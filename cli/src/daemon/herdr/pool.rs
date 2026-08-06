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
/// session is written off — the TUI never took it.
const SUBMIT_PATIENCE: u32 = 3;

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
}

impl HerdrPool {
    /// Connect, hold herdr to the protocol pin, and adopt whatever sessions
    /// of this root's are still alive from a previous daemon.
    pub fn connect(
        root: &Path,
        cfg: &HerdrHarness,
        max_concurrent: usize,
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
            if agent.tokens.get("trellis_root") != Some(&root) || self.in_flight.contains_key(&key)
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
        writeln!(file, "# prompt: {prompt}")?;

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
            self.client
                .agent_start(&agent_name(key), &self.cfg.kind, &pane_id, args, START)?;
            let mut attempt = 0;
            loop {
                match self.client.agent_prompt(&pane_id, prompt) {
                    Err(e)
                        if attempt < PROMPT_RETRIES
                            && e.to_string().contains("agent_not_ready") =>
                    {
                        attempt += 1;
                        std::thread::sleep(PROMPT_BACKOFF);
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
    /// for an operator with a terminal.
    pub fn drain(&mut self) -> Vec<Finished> {
        let mut done = Vec::new();
        for key in self.in_flight.keys().cloned().collect::<Vec<_>>() {
            let target = self.in_flight[&key].pane_id.clone();
            let mut rounds = 0;
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
                Ok(_) => done.push(self.finish(&key, Some(0))),
                Err(_) => done.push(self.finish(&key, None)),
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
        "idle" | "done" => match session.phase {
            // A turn was seen, and it ended.
            Phase::Working | Phase::Blocked | Phase::BlockedPending => Some(Some(0)),
            // Never seen working — but the sequence moved past the prompt,
            // so the turn ran between ticks.
            Phase::Submitted if agent.state_change_seq > session.seq_at_prompt => Some(Some(0)),
            Phase::Submitted => submit_patience(session),
        },
        // `unknown`, or a state this pin has never heard of: not an
        // observation. A session mid-turn survives a detection gap; one that
        // never started does not get to hide in it.
        _ => match session.phase {
            Phase::Submitted => submit_patience(session),
            _ => None,
        },
    }
}

fn submit_patience(session: &mut Session) -> Option<Option<i32>> {
    session.submitted_ticks += 1;
    if session.submitted_ticks >= SUBMIT_PATIENCE {
        note(&format!(
            "{}: prompt was never taken up — writing the session off",
            session.label
        ));
        return Some(None);
    }
    None
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
