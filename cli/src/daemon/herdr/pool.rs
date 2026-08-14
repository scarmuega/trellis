//! The herdr session backend: one workspace per session, an interactive
//! agent in its root pane, the prompt submitted to the running TUI.
//!
//! The lifecycle inverts the spawner's. A process exits and reports a code;
//! an interactive agent settles — `working` back to `idle` — and the pool
//! reads that settling as completion, one `agent.get` per session per tick.
//! Exit codes are therefore a rendering: a settled turn reports 0, a session
//! that vanished reports none, and the truth about the *plan* stays where it
//! always was — in the status its taker flips on disk (decision 0029).
//!
//! What this buys over the spawner: the session is a pane — attachable,
//! visible in herdr's sidebar, and alive across a daemon restart, where
//! `connect` re-adopts anything carrying this root's tokens. What it costs
//! is recorded in decision 0043, the budget cap first among the costs.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::client::{AgentInfo, Client};
use crate::daemon::config::{HerdrHarness, OnShutdown, Retain, RuntimeConfig};
use crate::daemon::note;
use crate::daemon::spawn::{shell_quote, slug, timestamp, Finished, InFlightView};
use crate::dates;

/// How long `agent.start` may take to reach an interactive prompt.
const START: Duration = Duration::from_secs(60);

/// How long `drain` will wait out one session. Not infinite, deliberately:
/// a `--once` pass that a harness can hang forever is a cron entry that
/// never reports.
const WAIT: Duration = Duration::from_secs(6 * 60 * 60);

/// Ticks a submitted prompt may sit with no observed state change before the
/// prompt is resubmitted — the TUI never took it.
const SUBMIT_PATIENCE: u32 = 3;

/// Times a dropped prompt is submitted again before the session is written
/// off. Observed live: herdr accepts `agent.prompt` while Claude Code is
/// still starting up, and the injected text vanishes — the pane settles
/// `idle` over an empty input, indistinguishable by status alone from a turn
/// that ran to completion between ticks.
const RESUBMITS: u32 = 2;

/// What `drain` waits before putting a dropped prompt back in. The tick loop
/// gets this spacing for free; a drain sees the startup idle within seconds
/// of spawning, and resubmitting into the same startup window that dropped
/// the first prompt would burn the whole budget on one dead zone.
const RESUBMIT_BEAT: Duration = Duration::from_secs(10);

/// Lines of scrollback read to verify a prompt was taken up. Deeper than the
/// retirement snapshot: the echo sits at the top of a turn's transcript, and
/// a turn that scrolled it past this window would read as never-started.
const VERIFY_TAIL: u32 = 10_000;

/// Lines of scrollback the harness's own status furniture occupies — the
/// footer and the turn's closing line, where the live-background-work
/// counters sit. Read narrowly on purpose: the same words higher up are a
/// session's prose, not a claim about what is still running.
const FOOTER_TAIL: u32 = 16;

/// Consecutive ticks the herdr server may be unreachable before every session
/// it was carrying is written off. The server dying kills its panes, so this
/// is acknowledgment, not abandonment.
const SERVER_PATIENCE: u32 = 3;

/// A freshly started agent may take a beat to register as promptable —
/// observed live: `agent.start` returns ready and an immediate prompt is
/// still refused `agent_not_ready`. Retry over ten seconds before giving up.
const PROMPT_RETRIES: u32 = 20;
const PROMPT_BACKOFF: Duration = Duration::from_millis(500);

/// Blocked must be seen twice before it is believed. Observed live: the
/// moment after a prompt is submitted can read `blocked` and settle into the
/// turn a beat later — a toast on the first sighting would cry wolf.
const BLOCKED_DEBOUNCE: Duration = Duration::from_secs(3);

/// Lines of scrollback copied into the session's log at retirement.
const TAIL: u32 = 500;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Phase {
    /// Prompt submitted; no turn observed yet.
    Submitted,
    Working,
    /// Blocked once; one more sighting makes it real.
    BlockedPending,
    /// Waiting at its own UI for a human. Holds its slot: the pressure on
    /// capacity is real, and hiding it would misreport the fleet.
    Blocked,
    /// Settled at its prompt with background monitors still armed: the turn
    /// ended, the session did not — a monitor is a standing promise to
    /// re-invoke it when its condition fires. Holds its slot for the same
    /// reason `Blocked` does, and for the same reason is never retired here.
    Waiting,
}

struct Session {
    workspace_id: String,
    pane_id: String,
    label: String,
    log: String,
    started: String,
    token: Option<String>,
    phase: Phase,
    /// `state_change_seq` when the prompt went in; a settle is only a finish
    /// if the sequence moved past it.
    seq_at_prompt: u64,
    /// Ticks spent in `Submitted` with nothing observed.
    submitted_ticks: u32,
    /// What went in, kept for two jobs: proving it landed (its echo in the
    /// scrollback) and submitting it again when it did not. Empty for an
    /// adopted session, whose prompt predates this daemon.
    prompt: String,
    /// Resubmissions spent against `RESUBMITS`.
    resubmits: u32,
}

impl Session {
    /// How herdr is addressed about this session. The pane is the identity —
    /// names can collide with a retained workspace's agent, panes cannot.
    fn target(&self) -> &str {
        &self.pane_id
    }
}

pub struct HerdrPool {
    root: PathBuf,
    log_dir: PathBuf,
    cfg: HerdrHarness,
    max_concurrent: usize,
    client: Client,
    in_flight: HashMap<String, Session>,
    /// `--once`: each task at most once per pool lifetime (`spawn.rs` has the
    /// argument).
    ledger: Option<HashSet<String>>,
    /// Consecutive reap passes that could not reach herdr at all.
    unreachable_ticks: u32,
    /// This process's keyspace (decision 0046): adoption takes only its own.
    key_prefix: &'static str,
}

impl HerdrPool {
    /// Connect, hold herdr to the protocol pin, and adopt whatever sessions
    /// of this root's are still alive from a previous daemon.
    pub fn connect(
        root: &Path,
        cfg: &HerdrHarness,
        max_concurrent: usize,
        key_prefix: &'static str,
    ) -> anyhow::Result<HerdrPool> {
        let client = Client::new(cfg.socket.as_deref());
        let pong = client.handshake().map_err(|e| {
            anyhow::anyhow!(
                "harness.backend is \"herdr\" but {e:#} — start it, or set backend = \"process\""
            )
        })?;
        note(&format!(
            "herdr backend: {} (protocol {}) at {}",
            pong.version,
            pong.protocol,
            client.socket().display()
        ));
        let mut pool = HerdrPool {
            root: root.to_path_buf(),
            log_dir: RuntimeConfig::runtime_dir(root).join("logs"),
            cfg: cfg.clone(),
            max_concurrent,
            client,
            in_flight: HashMap::new(),
            ledger: None,
            unreachable_ticks: 0,
            key_prefix,
        };
        pool.adopt()?;
        Ok(pool)
    }

    pub fn once_per_task(mut self) -> HerdrPool {
        self.ledger = Some(HashSet::new());
        self
    }

    /// Re-enter sessions a previous daemon left running (`on_shutdown =
    /// "detach"`). Identity is the tokens `spawn` stamps on the pane; a
    /// session found working or blocked is busy again — the guard the
    /// process backend never had across restarts — and one found settled is
    /// collected by the first tick's reap.
    fn adopt(&mut self) -> anyhow::Result<()> {
        let root = self.root.display().to_string();
        for agent in self.client.agent_list()? {
            let Some(key) = agent.tokens.get("trellis_key").cloned() else {
                continue;
            };
            if agent.tokens.get("trellis_root") != Some(&root)
                || !key.starts_with(self.key_prefix)
                || self.in_flight.contains_key(&key)
            {
                continue;
            }
            let label = agent
                .tokens
                .get("trellis_label")
                .cloned()
                .unwrap_or_else(|| key.clone());
            note(&format!(
                "adopted {label} from herdr pane {} ({})",
                agent.pane_id, agent.agent_status
            ));
            self.in_flight.insert(
                key,
                Session {
                    workspace_id: agent.workspace_id,
                    pane_id: agent.pane_id,
                    label,
                    log: agent.tokens.get("trellis_log").cloned().unwrap_or_default(),
                    started: timestamp(dates::today()),
                    // The previous daemon's channel died with it; an adopted
                    // session runs, but can no longer ask.
                    token: None,
                    phase: match agent.agent_status.as_str() {
                        // Confirmed by the next tick's own sighting.
                        "blocked" => Phase::BlockedPending,
                        _ => Phase::Working,
                    },
                    // Unknown, and unneeded: an adopted session already moved
                    // past its prompt, so any settle is a finish.
                    seq_at_prompt: 0,
                    submitted_ticks: 0,
                    prompt: String::new(),
                    resubmits: 0,
                },
            );
        }
        Ok(())
    }

    pub fn already_ran(&self, key: &str) -> bool {
        self.ledger.as_ref().is_some_and(|l| l.contains(key))
    }

    pub fn is_busy(&self, key: &str) -> bool {
        self.in_flight.contains_key(key)
    }

    pub fn capacity(&self) -> usize {
        self.max_concurrent.saturating_sub(self.in_flight.len())
    }

    pub fn in_flight(&self) -> Vec<InFlightView> {
        let mut views: Vec<InFlightView> = self
            .in_flight
            .iter()
            .map(|(key, s)| InFlightView {
                key: key.clone(),
                label: match s.phase {
                    Phase::Blocked => format!("{} [blocked]", s.label),
                    Phase::Waiting => format!("{} [waiting]", s.label),
                    _ => s.label.clone(),
                },
                log: s.log.clone(),
                started: s.started.clone(),
                token: s.token.clone(),
            })
            .collect();
        views.sort_by(|a, b| a.key.cmp(&b.key));
        views
    }

    /// Launch one session: a workspace at the root, identity tokens on its
    /// pane, the agent started, the prompt submitted. Returns the log name,
    /// like the spawner — the log starts as a header and receives the pane's
    /// scrollback at retirement.
    pub fn spawn(
        &mut self,
        key: &str,
        label: &str,
        args: &[String],
        prompt: &str,
        token: Option<String>,
    ) -> anyhow::Result<String> {
        let stamp = timestamp(dates::today());
        let log = format!("{stamp}-{}.log", slug(key));
        std::fs::create_dir_all(&self.log_dir)?;
        let mut file = std::fs::File::create(self.log_dir.join(&log))?;
        writeln!(file, "# {label}")?;
        writeln!(
            file,
            "# herdr agent: {} {}",
            self.cfg.kind,
            shell_quote(args)
        )?;
        writeln!(file, "# prompt: {}", prompt.replace('\n', "\n# "))?;

        let (workspace_id, pane_id) = self.client.workspace_create(&self.root, label)?;

        // Everything after the workspace exists tears it down on failure: a
        // half-started session left on screen would look like work.
        let started = (|| -> anyhow::Result<AgentInfo> {
            self.client.pane_report_tokens(
                &pane_id,
                &[
                    ("trellis_key", key),
                    ("trellis_root", &self.root.display().to_string()),
                    ("trellis_label", label),
                    ("trellis_log", &log),
                ],
            )?;
            // Observed live: a freshly created pane can refuse `agent.start`
            // with `agent_pane_busy` while its shell is still coming up —
            // the same startup seam `agent_not_ready` is on the prompt side.
            // Same medicine: retry over the backoff window before failing.
            let running = {
                let mut attempt = 0;
                loop {
                    match self.client.agent_start(
                        &agent_name(key),
                        &self.cfg.kind,
                        &pane_id,
                        args,
                        START,
                    ) {
                        Err(e)
                            if attempt < PROMPT_RETRIES
                                && e.to_string().contains("agent_pane_busy") =>
                        {
                            attempt += 1;
                            std::thread::sleep(PROMPT_BACKOFF);
                        }
                        other => break other?,
                    }
                }
            };
            let mut attempt = 0;
            loop {
                match self.client.agent_prompt(&pane_id, prompt) {
                    Err(e) if e.to_string().contains("agent_not_ready") => {
                        if attempt < PROMPT_RETRIES {
                            attempt += 1;
                            std::thread::sleep(PROMPT_BACKOFF);
                            continue;
                        }
                        // Still not promptable — a concurrent cold start can
                        // outlast the retries. The agent runs; hand the
                        // session over with the prompt unplaced and let the
                        // resubmission machinery put it in.
                        note(&format!(
                            "{label}: agent not promptable yet — the prompt will be resubmitted"
                        ));
                        break Ok(running);
                    }
                    other => break other,
                }
            }
        })();

        let agent = match started {
            Ok(agent) => agent,
            Err(e) => {
                let _ = self.client.workspace_close(&workspace_id);
                return Err(e);
            }
        };

        writeln!(file, "# herdr workspace {workspace_id}, pane {pane_id}")?;
        if let Some(ledger) = &mut self.ledger {
            ledger.insert(key.to_string());
        }
        self.in_flight.insert(
            key.to_string(),
            Session {
                workspace_id,
                pane_id,
                label: label.to_string(),
                log: log.clone(),
                started: stamp,
                token,
                phase: Phase::Submitted,
                seq_at_prompt: agent.state_change_seq,
                submitted_ticks: 0,
                prompt: prompt.to_string(),
                resubmits: 0,
            },
        );
        Ok(log)
    }

    /// One `agent.get` per session: advance each little state machine,
    /// collect the settled. Never blocks beyond the socket timeout.
    pub fn reap(&mut self) -> Vec<Finished> {
        let mut done = Vec::new();
        let mut unreachable = false;

        for key in self.in_flight.keys().cloned().collect::<Vec<_>>() {
            let session = self.in_flight.get_mut(&key).expect("still tracked");
            let agent = match self.client.agent_get(session.target()) {
                Ok(agent) => agent,
                Err(e) if is_unreachable(&e) => {
                    unreachable = true;
                    continue;
                }
                // Herdr answered and does not know the pane: the session is
                // gone — closed under us, or its workspace with it.
                Err(_) => {
                    done.push(self.finish(&key, None));
                    continue;
                }
            };
            if let Some(exit) = observe(session, &agent, &self.client) {
                done.push(self.finish(&key, exit));
            }
        }

        // The server being down is one fact, not one per session. Sessions
        // outlive a daemon restart but not their own server: after enough
        // silence, acknowledge the panes went with it.
        if unreachable && !self.in_flight.is_empty() {
            self.unreachable_ticks += 1;
            if self.unreachable_ticks == 1 {
                note("herdr is unreachable — sessions kept, will retry");
            }
            if self.unreachable_ticks >= SERVER_PATIENCE {
                note(&format!(
                    "herdr unreachable for {SERVER_PATIENCE} passes — writing its sessions off"
                ));
                for key in self.in_flight.keys().cloned().collect::<Vec<_>>() {
                    done.push(self.finish(&key, None));
                }
            }
        } else {
            self.unreachable_ticks = 0;
        }

        done.sort_by(|a, b| a.key.cmp(&b.key));
        done
    }

    /// Block until every session settles — what `--once` owes its caller. A
    /// session that settles `blocked` is not waited out: a drain pass has
    /// nobody to unblock it, so it is announced, detached, and left running
    /// for an operator with a terminal. One settled with background monitors
    /// armed is detached the same way, for the mirror reason: it needs nobody,
    /// and the clock it is waiting on is not the drain's to spend.
    pub fn drain(&mut self) -> Vec<Finished> {
        let mut done = Vec::new();
        for key in self.in_flight.keys().cloned().collect::<Vec<_>>() {
            let target = self.in_flight[&key].pane_id.clone();
            let mut rounds = 0;
            'session: loop {
                let settled = loop {
                    match self.client.agent_wait(&target, &["idle", "done", "blocked"], WAIT) {
                        // Blocked must survive the debounce to be believed —
                        // the moment after submission can read blocked and
                        // settle into the turn a beat later.
                        Ok(agent) if agent.agent_status == "blocked" && rounds < 100 => {
                            rounds += 1;
                            std::thread::sleep(BLOCKED_DEBOUNCE);
                            match self.client.agent_get(&target) {
                                Ok(again) if again.agent_status == "blocked" => break Ok(again),
                                Ok(_) => continue,
                                Err(e) => break Err(e),
                            }
                        }
                        other => break other,
                    }
                };
                match settled {
                    Ok(agent) if agent.agent_status == "blocked" => {
                        let session = self.in_flight.remove(&key).expect("still tracked");
                        note(&format!(
                            "{} is blocked in herdr — left running; attach to unblock it",
                            session.label
                        ));
                        let _ = self.client.notification_show(
                            &format!("session blocked: {}", session.label),
                            Some("left running — attach in herdr to unblock it"),
                        );
                    }
                    // Settled with monitors still armed: the session will
                    // resume itself, on a clock a drain has no business
                    // waiting out — a monitor may be minding an hour-long
                    // download. Announced and left running, like a blocked
                    // one; the next daemon adopts it.
                    Ok(_) if monitors_live(&self.in_flight[&key], &self.client) => {
                        let session = self.in_flight.remove(&key).expect("still tracked");
                        note(&format!(
                            "{} is idle with background monitors armed — left running; \
                             it resumes itself when one fires",
                            session.label
                        ));
                        let _ = self.client.notification_show(
                            &format!("session waiting: {}", session.label),
                            Some("left running — a background monitor will resume it"),
                        );
                    }
                    Ok(_) => {
                        // A freshly spawned agent reads settled before it has
                        // taken any prompt at all — the wait returns on the
                        // startup idle, not a finished turn. Same seam the
                        // tick path verifies: no echo, no finish.
                        let session = self.in_flight.get_mut(&key).expect("still tracked");
                        if session.phase == Phase::Submitted
                            && !prompt_landed(session, &self.client)
                        {
                            // Let the startup window that dropped the first
                            // prompt pass before spending another one.
                            std::thread::sleep(RESUBMIT_BEAT);
                            match resubmit(session, &self.client) {
                                None => continue 'session,
                                Some(exit) => done.push(self.finish(&key, exit)),
                            }
                        } else if settle_survives(session, &self.client) {
                            done.push(self.finish(&key, Some(0)));
                        } else {
                            // Not settled after all — a dialog or a next
                            // turn showed up in the debounce; wait it out.
                            continue 'session;
                        }
                    }
                    Err(_) => done.push(self.finish(&key, None)),
                }
                break 'session;
            }
        }
        done.sort_by(|a, b| a.key.cmp(&b.key));
        done
    }

    /// The shutdown half of the policy. `stop` closes every workspace and
    /// reports the sessions as signalled; `detach` reports nothing and
    /// leaves them for `adopt`.
    pub fn stop_all(&mut self) -> (usize, Vec<Finished>) {
        match self.cfg.on_shutdown {
            OnShutdown::Detach => {
                if !self.in_flight.is_empty() {
                    note(&format!(
                        "leaving {} session(s) running in herdr — the next daemon adopts them",
                        self.in_flight.len()
                    ));
                }
                self.in_flight.clear();
                (0, Vec::new())
            }
            OnShutdown::Stop => {
                let mut done = Vec::new();
                for key in self.in_flight.keys().cloned().collect::<Vec<_>>() {
                    done.push(self.finish_closing(&key, None, true));
                }
                done.sort_by(|a, b| a.key.cmp(&b.key));
                (done.len(), done)
            }
        }
    }

    /// Retire one session: snapshot its scrollback into the log, close or
    /// keep its workspace per `retain`, and hand back the `Finished` the
    /// daemon's bookkeeping expects.
    fn finish(&mut self, key: &str, exit: Option<i32>) -> Finished {
        let close = match self.cfg.retain {
            Retain::Always => false,
            Retain::Never => true,
            // Keep the scene of a failure for attach; clear the clean ones.
            Retain::OnFailure => exit == Some(0),
        };
        self.finish_closing(key, exit, close)
    }

    fn finish_closing(&mut self, key: &str, exit: Option<i32>, close: bool) -> Finished {
        let session = self.in_flight.remove(key).expect("finishing a tracked key");
        if !session.log.is_empty() {
            if let Ok(tail) = self.client.agent_read(&session.pane_id, TAIL) {
                let _ = append_tail(&self.log_dir.join(&session.log), &tail);
            }
        }
        if close {
            let _ = self.client.workspace_close(&session.workspace_id);
        }
        Finished {
            key: key.to_string(),
            label: session.label,
            exit,
            token: session.token,
        }
    }
}

/// Advance one session's phase from one observation. `Some(exit)` means it
/// settled and should be retired.
fn observe(session: &mut Session, agent: &AgentInfo, client: &Client) -> Option<Option<i32>> {
    match agent.agent_status.as_str() {
        "working" => {
            if session.phase == Phase::Waiting {
                note(&format!(
                    "{}: a monitor fired — working again",
                    session.label
                ));
            }
            session.phase = Phase::Working;
            session.submitted_ticks = 0;
            None
        }
        // Blocked from any phase — including straight from submission, which
        // is what a trust or permission dialog at startup looks like. Seen
        // once it is pending; seen twice it is real, and toasts once.
        "blocked" => {
            match session.phase {
                Phase::Blocked => {}
                Phase::BlockedPending => {
                    session.phase = Phase::Blocked;
                    note(&format!(
                        "{} is blocked in herdr — attach to unblock it",
                        session.label
                    ));
                    let _ = client.notification_show(
                        &format!("session blocked: {}", session.label),
                        Some("attach in herdr to unblock it"),
                    );
                }
                _ => session.phase = Phase::BlockedPending,
            }
            None
        }
        // Idle is not done while the harness still reports armed monitors:
        // the turn ended, the session did not. Checked before every other
        // reading of a settle, including the prompt-echo one — a session with
        // live monitors has taken a prompt by construction.
        "idle" | "done" if monitors_live(session, client) => {
            if session.phase != Phase::Waiting {
                session.phase = Phase::Waiting;
                note(&format!(
                    "{}: idle with background monitors armed — waiting, not finished; \
                     it re-invokes itself when one fires",
                    session.label
                ));
            }
            None
        }
        "idle" | "done" => match session.phase {
            // A turn was seen, and it ended — if the settle is real.
            // Observed live: a permission dialog rendered mid-turn can read
            // `idle` for a beat before the classifier calls it `blocked`,
            // and retiring on that beat closes the workspace over a waiting
            // dialog. A settle counts only after it survives the debounce.
            Phase::Working | Phase::Blocked | Phase::BlockedPending | Phase::Waiting => {
                if settle_survives(session, client) {
                    Some(Some(0))
                } else {
                    None
                }
            }
            // Never seen working, and the sequence moved past the prompt.
            // That is not proof of a turn: startup state flaps move the
            // sequence too, and an agent that dropped the injection while
            // still starting up settles exactly like one that finished
            // between ticks. Only the prompt's echo in the pane tells them
            // apart — and the settle still has to survive the debounce.
            Phase::Submitted if agent.state_change_seq > session.seq_at_prompt => {
                if !prompt_landed(session, client) {
                    resubmit(session, client)
                } else if settle_survives(session, client) {
                    Some(Some(0))
                } else {
                    None
                }
            }
            Phase::Submitted => submit_patience(session, client),
        },
        // `unknown`, or a state this pin has never heard of: not an
        // observation. A session mid-turn survives a detection gap; one that
        // never started does not get to hide in it.
        _ => match session.phase {
            Phase::Submitted => submit_patience(session, client),
            _ => None,
        },
    }
}

fn submit_patience(session: &mut Session, client: &Client) -> Option<Option<i32>> {
    session.submitted_ticks += 1;
    if session.submitted_ticks >= SUBMIT_PATIENCE {
        return resubmit(session, client);
    }
    None
}

/// A settle believed once is a settle believed too early: re-read after the
/// debounce and require it to still be settled. A dialog or a next turn
/// showing up in the gap means the session is not done — the observation is
/// discarded and the next tick classifies it properly (a dialog reads
/// `blocked` by then, a turn `working`). On a read error, err toward not
/// retiring: the session survives to the next tick.
fn settle_survives(session: &Session, client: &Client) -> bool {
    std::thread::sleep(BLOCKED_DEBOUNCE);
    match client.agent_get(session.target()) {
        Ok(again) => matches!(again.agent_status.as_str(), "idle" | "done"),
        Err(_) => false,
    }
}

/// Whether the harness still reports armed background monitors. Claude Code
/// carries the count in its footer ("⏵⏵ auto mode on · 2 shells, 1 monitor")
/// and in the turn's closing line ("· 2 shells, 1 monitor still running"),
/// which is why only the footer window is read.
///
/// Background *shells* are deliberately not counted. A monitor is a promise
/// to re-invoke the session; a shell is a process that may outlive the turn
/// that started it — a dev server left running would hold a slot forever.
///
/// On a read error, err toward finishing: the session is judged exactly as it
/// was before this check existed, rather than parked on an unreadable pane.
fn monitors_live(session: &Session, client: &Client) -> bool {
    match client.agent_read(session.target(), FOOTER_TAIL) {
        Ok(tail) => monitors_in(&tail) > 0,
        Err(_) => false,
    }
}

/// The largest `N monitor(s)` count the text reports, zero if it names none.
/// A hand parse rather than a pattern: the phrase is two tokens, and this
/// runs once per settled session per tick.
fn monitors_in(text: &str) -> u32 {
    let mut most = 0;
    for (at, _) in text.match_indices("monitor") {
        let before = text[..at].trim_end_matches(' ');
        let digits = before.trim_end_matches(|c: char| c.is_ascii_digit());
        // "1 monitor", never "the monitor" — a bare word is prose.
        if digits.len() == before.len() {
            continue;
        }
        if let Ok(n) = before[digits.len()..].parse::<u32>() {
            most = most.max(n);
        }
    }
    most
}

/// Whether the pane carries the prompt's echo — the observable difference
/// between a turn that ran and an injection the agent dropped. Matched on a
/// prefix: the pane wraps long lines. An adopted session has no prompt to
/// verify; its settle stays a finish.
fn prompt_landed(session: &Session, client: &Client) -> bool {
    if session.prompt.is_empty() {
        return true;
    }
    match client.agent_read(&session.pane_id, VERIFY_TAIL) {
        Ok(tail) => tail.contains(prompt_needle(&session.prompt)),
        Err(_) => false,
    }
}

/// Enough of the prompt to be unmistakable in scrollback, short enough to
/// survive line wrapping at any plausible pane width. Cut at the first line
/// before the character cut: the default prompts are multiline and open with
/// a per-session-unique header (decision 0050), and a needle carrying a
/// newline would never match wrapped scrollback.
fn prompt_needle(prompt: &str) -> &str {
    let first_line = prompt.split('\n').next().unwrap_or(prompt);
    match first_line.char_indices().nth(24) {
        Some((i, _)) => &first_line[..i],
        None => first_line,
    }
}

/// Put a dropped prompt back in, up to `RESUBMITS` times; past the budget the
/// session is written off. On success the watermarks reset, so the next
/// settle is judged against the new submission.
fn resubmit(session: &mut Session, client: &Client) -> Option<Option<i32>> {
    if session.resubmits >= RESUBMITS {
        note(&format!(
            "{}: prompt was never taken up — writing the session off",
            session.label
        ));
        return Some(None);
    }
    session.resubmits += 1;
    match client.agent_prompt(&session.pane_id, &session.prompt) {
        Ok(agent) => {
            note(&format!(
                "{}: prompt was dropped at startup — resubmitted ({}/{RESUBMITS})",
                session.label, session.resubmits
            ));
            session.seq_at_prompt = agent.state_change_seq;
            session.submitted_ticks = 0;
            None
        }
        // Not fatal, and not placed either: the agent is still starting up.
        // The attempt still spends budget, so a pane that never becomes
        // promptable runs out the same clock as one that keeps dropping.
        Err(e) if e.to_string().contains("agent_not_ready") => {
            note(&format!(
                "{}: agent not promptable yet — will retry ({}/{RESUBMITS})",
                session.label, session.resubmits
            ));
            session.submitted_ticks = 0;
            None
        }
        Err(e) => {
            note(&format!(
                "{}: prompt was dropped and resubmission failed ({e:#}) — writing the session off",
                session.label
            ));
            Some(None)
        }
    }
}

/// A herdr-legal agent name for a task key: lowercase letters, digits, `-`
/// and `_`, starting with a letter, at most 32 characters — herdr refuses
/// anything else. A display label, not an identity: the pane is the identity.
fn agent_name(key: &str) -> String {
    let mut name = String::from("t-");
    for c in key.chars() {
        name.push(match c {
            'a'..='z' | '0'..='9' | '-' | '_' => c,
            'A'..='Z' => c.to_ascii_lowercase(),
            _ => '-',
        });
        if name.len() == 32 {
            break;
        }
    }
    name.trim_end_matches('-').to_string()
}

fn append_tail(path: &Path, tail: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    writeln!(file, "\n# --- pane scrollback at retirement ---")?;
    writeln!(file, "{tail}")
}

/// A connect failure, as opposed to herdr answering with a refusal.
fn is_unreachable(e: &anyhow::Error) -> bool {
    e.to_string().contains("cannot reach herdr")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A one-shot fake, the shape `client.rs` tests against: one scripted
    /// reply per connection.
    fn client(dir: &Path, replies: Vec<String>) -> Client {
        use std::io::{BufRead, BufReader};
        use std::os::unix::net::UnixListener;

        let path = dir.join("herdr.sock");
        let listener = UnixListener::bind(&path).unwrap();
        std::thread::spawn(move || {
            for reply in replies {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                let mut line = String::new();
                let _ = BufReader::new(&stream).read_line(&mut line);
                let id = serde_json::from_str::<serde_json::Value>(&line)
                    .map(|r| r["id"].as_str().unwrap_or("?").to_string())
                    .unwrap_or_default();
                let mut writer = &stream;
                let _ = writeln!(writer, "{}", reply.replace("{id}", &id));
            }
        });
        Client::new(Some(path.to_str().unwrap()))
    }

    fn reads(text: &str) -> String {
        format!(r#"{{"id":"{{id}}","result":{{"type":"pane_read","read":{{"text":"{text}"}}}}}}"#)
    }

    fn session() -> Session {
        Session {
            workspace_id: "w1".into(),
            pane_id: "p1".into(),
            label: "act plans/x.md → org/founder".into(),
            log: String::new(),
            started: "2026-08-14T0000".into(),
            token: None,
            phase: Phase::Working,
            seq_at_prompt: 1,
            submitted_ticks: 0,
            prompt: "plans/x.md — dispatched act".into(),
            resubmits: 0,
        }
    }

    fn sighting(status: &str) -> AgentInfo {
        AgentInfo {
            pane_id: "p1".into(),
            workspace_id: "w1".into(),
            agent_status: status.into(),
            state_change_seq: 9,
            interactive_ready: true,
            name: None,
            tokens: Default::default(),
        }
    }

    /// The live failure: a settled pane whose harness still reports armed
    /// monitors. Retiring here closed a workspace over work in flight.
    #[test]
    fn an_idle_pane_with_armed_monitors_waits_instead_of_finishing() {
        let dir = tempfile::TempDir::new().unwrap();
        let client = client(
            dir.path(),
            vec![reads("... 2 shells, 1 monitor still running")],
        );
        let mut session = session();

        assert!(observe(&mut session, &sighting("idle"), &client).is_none());
        assert_eq!(session.phase, Phase::Waiting);

        // The monitor fires: back to work, and no read is spent to see it.
        assert!(observe(&mut session, &sighting("working"), &client).is_none());
        assert_eq!(session.phase, Phase::Working);
    }

    /// The same settle with nothing armed still finishes — the check adds a
    /// reason to wait, never a reason to hang.
    #[test]
    fn an_idle_pane_with_nothing_armed_still_settles() {
        let dir = tempfile::TempDir::new().unwrap();
        let client = client(
            dir.path(),
            vec![
                reads("⏵⏵ auto mode on · ctrl+t to hide tasks"),
                // What the settle debounce re-reads.
                r#"{"id":"{id}","result":{"type":"agent_info","agent":{"pane_id":"p1","workspace_id":"w1","agent_status":"idle"}}}"#.to_string(),
            ],
        );
        let mut session = session();
        assert_eq!(
            observe(&mut session, &sighting("idle"), &client),
            Some(Some(0))
        );
    }

    #[test]
    fn the_footer_counts_monitors_and_ignores_everything_else() {
        // The live footer that started this: a settled pane, work still in
        // motion behind it.
        assert_eq!(
            monitors_in("  ⏵⏵ auto mode on · 2 shells, 1 monitor · ctrl+t to hide tasks"),
            1
        );
        assert_eq!(
            monitors_in("✻ Brewed for 31m 28s · 2 shells, 1 monitor still running"),
            1
        );
        assert_eq!(monitors_in("· 12 monitors still running"), 12);

        // Nothing armed: a plain finish, and prose that merely says the word.
        assert_eq!(monitors_in("⏵⏵ auto mode on · 2 shells · ctrl+t"), 0);
        assert_eq!(
            monitors_in("I will monitor the rollout and report back."),
            0
        );
        assert_eq!(monitors_in("the monitor exited"), 0);
    }

    #[test]
    fn the_needle_is_the_first_line_and_never_carries_a_newline() {
        let prompt = "plans/x.md — dispatched act as coder (trellis runtime).\n\nAct as coder…";
        let needle = prompt_needle(prompt);
        assert!(!needle.contains('\n'), "{needle:?}");
        assert!(prompt.starts_with(needle));
        assert_eq!(needle.chars().count(), 24);

        assert_eq!(prompt_needle("short\nrest"), "short");
        assert_eq!(prompt_needle("bare"), "bare");
    }
}
