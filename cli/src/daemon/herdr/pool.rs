//! The herdr session pool: one workspace per session, an interactive agent
//! in its root pane, the prompt submitted to the running TUI.
//!
//! **Completion is read from disk, not from the screen** (decision 0061). A
//! pane going idle is not an event — it is a question, and the answer lives
//! in the plan the session was sent to advance: a verdict there (retired,
//! blocked, passed on, or parked behind a declared `handoff:`) means the
//! session is done, and an idle pane still holding an unaccounted-for plan
//! is a stall, graced and then recycled. The pool never reads a plan itself
//! — it asks its caller, through `Disposition` — so the model stays the
//! daemon's and the panes stay this file's.
//!
//! What that buys is the absence of a machine: no prompt-echo matching, no
//! status-footer parsing, no settle debounce, no resubmission budget. Every
//! one of those existed to manufacture a completion event this pool no
//! longer needs, and each was a reading of another program's screen.
//!
//! What it costs is latency. A session whose prompt was dropped at startup
//! looks exactly like one that stalled, and is recovered the same way —
//! after the idle grace, and then the retry cooldown — rather than in the
//! seconds a resubmission took. Decision 0061 takes that trade deliberately.
//!
//! The session is a pane: attachable, visible in herdr's sidebar, and alive
//! across a daemon restart, where `connect` re-adopts anything carrying this
//! root's tokens.

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use super::client::{AgentInfo, Client};
use crate::daemon::config::{HerdrHarness, OnShutdown, Retain, RuntimeConfig};
use crate::daemon::note;
use crate::daemon::session::{
    append_tail, shell_quote, slug, timestamp, Finished, InFlightView, Reason,
};
use crate::dates;

/// How long `agent.start` may take to reach an interactive prompt.
const START: Duration = Duration::from_secs(60);

/// How long `drain` will wait out the fleet. Not infinite, deliberately: a
/// `--once` pass that a harness can hang forever is a cron entry that never
/// reports.
const DRAIN_DEADLINE: Duration = Duration::from_secs(6 * 60 * 60);

/// How often `drain` re-reads the fleet while waiting it out.
const DRAIN_BEAT: Duration = Duration::from_secs(1);

/// Consecutive idle sightings before a settle is acted on. Two rather than
/// one, and paced by the caller's tick rather than by a sleep: a permission
/// dialog rendered mid-turn reads `idle` for a beat before the classifier
/// calls it `blocked`, and a session that writes its verdict and then keeps
/// composing its report would otherwise be retired mid-sentence.
const IDLE_STREAK: u32 = 2;

/// Consecutive blocked sightings before the operator is told. Same reason,
/// mirrored: the moment after a prompt is submitted can read `blocked` and
/// settle into the turn a beat later, and a toast on the first sighting
/// would cry wolf.
const BLOCKED_STREAK: u32 = 2;

/// Consecutive passes the herdr server may be unreachable before every
/// session it was carrying is written off. The server dying kills its panes,
/// so this is acknowledgment, not abandonment.
const SERVER_PATIENCE: u32 = 3;

/// A freshly started agent may take a beat to register as promptable —
/// observed live: `agent.start` returns ready and an immediate prompt is
/// still refused `agent_not_ready`. Retry over ten seconds before giving up.
const PROMPT_RETRIES: u32 = 20;
const PROMPT_BACKOFF: Duration = Duration::from_millis(500);

/// Lines of scrollback copied into the session's log at retirement.
const TAIL: u32 = 500;

/// What the caller knows, from disk, about the task behind a session key.
/// The pool asks this only about a pane it has just seen settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// The plan has been accounted for: moved, retired, blocked, passed on,
    /// or parked behind a declared `handoff:`. A settled pane over one of
    /// these is a finished session.
    Verdict,
    /// The plan is still claimed with no verdict. A settled pane over one of
    /// these is a stall: graced, then recycled.
    Held,
    /// There is no plan behind this key — a ritual. Settling is all the
    /// completion such a session has.
    NoPlan,
}

struct Session {
    workspace_id: String,
    pane_id: String,
    label: String,
    log: String,
    started: String,
    token: Option<String>,
    /// Whether this session came from a previous daemon, which changes only
    /// what the log lines say — an adopted pane is judged like any other.
    adopted: bool,
    /// When the pane was first seen settled in its current spell of idleness;
    /// cleared the moment it works again.
    idle_since: Option<Instant>,
    /// Consecutive settled sightings, against `IDLE_STREAK`.
    idle_streak: u32,
    /// Consecutive blocked sightings, against `BLOCKED_STREAK`.
    blocked_streak: u32,
    /// Whether the blocked toast has already gone out for this spell.
    toasted: bool,
}

impl Session {
    /// How herdr is addressed about this session. The pane is the identity —
    /// names can collide with a retained workspace's agent, panes cannot.
    fn target(&self) -> &str {
        &self.pane_id
    }

    /// A pane waiting at its own UI holds its slot: the pressure on capacity
    /// is real, and hiding it would misreport the fleet.
    fn is_blocked(&self) -> bool {
        self.blocked_streak >= BLOCKED_STREAK
    }

    fn working(&mut self) {
        self.idle_since = None;
        self.idle_streak = 0;
        self.blocked_streak = 0;
        self.toasted = false;
    }
}

pub struct HerdrPool {
    root: PathBuf,
    log_dir: PathBuf,
    cfg: HerdrHarness,
    max_concurrent: usize,
    idle_grace: Duration,
    client: Client,
    in_flight: HashMap<String, Session>,
    /// `--once`: each task at most once per pool lifetime.
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
            anyhow::anyhow!("the runtime drives sessions through herdr but {e} — start it")
        })?;
        note(&format!(
            "herdr: {} (protocol {}) at {}",
            pong.version,
            pong.protocol,
            client.socket().display()
        ));
        let mut pool = HerdrPool {
            root: root.to_path_buf(),
            log_dir: RuntimeConfig::runtime_dir(root).join("logs"),
            cfg: cfg.clone(),
            max_concurrent,
            idle_grace: Duration::from_secs(cfg.idle_grace_secs),
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
    /// session found here is busy again — the guard the runtime never had
    /// across restarts — and one found settled is judged by the first pass
    /// like any other.
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
                    adopted: true,
                    idle_since: None,
                    idle_streak: 0,
                    // Confirmed by this pass's own sighting, like any other.
                    blocked_streak: u32::from(agent.agent_status == "blocked"),
                    toasted: false,
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
                label: if s.is_blocked() {
                    format!("{} [blocked]", s.label)
                } else {
                    s.label.clone()
                },
                log: s.log.clone(),
                started: s.started.clone(),
                token: s.token.clone(),
                // The pane is the session's identity here (see `target`),
                // and the one thing an operator needs to go look at it.
                pane: Some(s.pane_id.clone()),
            })
            .collect();
        views.sort_by(|a, b| a.key.cmp(&b.key));
        views
    }

    /// Launch one session: a workspace at the root, identity tokens on its
    /// pane, the agent started, the prompt submitted. Returns the log name;
    /// the log starts as a header and receives the pane's scrollback at
    /// retirement.
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
        let started = (|| -> anyhow::Result<()> {
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
            retry_while("agent_pane_busy", || {
                self.client
                    .agent_start(&agent_name(key), &self.cfg.kind, &pane_id, args, START)
            })?;
            // The prompt is submitted once. If the TUI drops it — which a
            // cold start can still do — the pane simply sits idle over a
            // claimed plan, and the idle grace recovers it (decision 0061).
            match retry_while("agent_not_ready", || {
                self.client.agent_prompt(&pane_id, prompt)
            }) {
                Ok(_) => Ok(()),
                Err(e) if e.code() == Some("agent_not_ready") => {
                    note(&format!(
                        "{label}: the agent never became promptable — the session will be \
                         recycled once its idle grace runs out"
                    ));
                    Ok(())
                }
                Err(e) => Err(e.into()),
            }
        })();

        if let Err(e) = started {
            let _ = self.client.workspace_close(&workspace_id);
            return Err(e);
        }

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
                adopted: false,
                idle_since: None,
                idle_streak: 0,
                blocked_streak: 0,
                toasted: false,
            },
        );
        Ok(log)
    }

    /// One `agent.get` per session, judged against what `judge` reads from
    /// disk. Never blocks beyond the socket timeout, and never sleeps.
    pub fn reap(&mut self, judge: &dyn Fn(&str) -> Disposition) -> Vec<Finished> {
        let mut done = Vec::new();
        let mut unreachable = false;

        for key in self.in_flight.keys().cloned().collect::<Vec<_>>() {
            let session = self.in_flight.get_mut(&key).expect("still tracked");
            let sighting = self.client.agent_get(session.target());
            let agent = match sighting {
                Ok(agent) => agent,
                // The server being unobservable says nothing about this
                // pane; it is counted once for the fleet, below.
                Err(e) if e.is_unreachable() => {
                    unreachable = true;
                    continue;
                }
                // Herdr answered and does not know the pane: the session is
                // gone — closed under us, or its workspace with it.
                Err(_) => {
                    done.push(self.finish(&key, Reason::Gone));
                    continue;
                }
            };
            if let Some(reason) = observe(session, &agent, judge(&key), self.idle_grace) {
                done.push(self.finish(&key, reason));
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
                    done.push(self.finish(&key, Reason::ServerLost));
                }
            }
        } else {
            self.unreachable_ticks = 0;
        }

        done.sort_by(|a, b| a.key.cmp(&b.key));
        done
    }

    /// Wait the fleet out — what `--once` owes its caller. Sessions that
    /// reach a verdict retire; sessions that stall are recycled once their
    /// grace runs out; sessions blocked at their own UI are announced and
    /// left running, because a drain has nobody to unblock them and the next
    /// daemon adopts them.
    pub fn drain(&mut self, judge: &dyn Fn(&str) -> Disposition) -> Vec<Finished> {
        let mut done = Vec::new();
        let deadline = Instant::now() + DRAIN_DEADLINE;
        loop {
            done.extend(self.reap(judge));
            let waiting = self.in_flight.values().filter(|s| !s.is_blocked()).count();
            if waiting == 0 || Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(DRAIN_BEAT);
        }
        // Whatever is still here is blocked, or outlasted the deadline.
        // Either way it keeps its marker line: it is running, and the next
        // daemon adopts it rather than judging its plan abandoned.
        for key in self.in_flight.keys().cloned().collect::<Vec<_>>() {
            let session = self.in_flight.get(&key).expect("still tracked");
            let (line, body) = if session.is_blocked() {
                (
                    format!(
                        "{} is blocked in herdr — left running; attach to unblock it",
                        session.label
                    ),
                    "left running — attach in herdr to unblock it",
                )
            } else {
                (
                    format!(
                        "{} outlasted the drain — left running; the next daemon adopts it",
                        session.label
                    ),
                    "left running — it outlasted a one-shot pass",
                )
            };
            note(&line);
            let _ = self.client.notification_show(
                &format!("session left running: {}", session.label),
                Some(body),
            );
            done.push(self.retire(&key, Reason::Detached));
        }
        done.sort_by(|a, b| a.key.cmp(&b.key));
        done
    }

    /// The shutdown half of the policy. `stop` closes every workspace and
    /// reports the sessions as stopped; `detach` reports nothing and leaves
    /// them for the next daemon's `adopt`.
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
                    done.push(self.finish_closing(&key, Reason::Stopped, true));
                }
                done.sort_by(|a, b| a.key.cmp(&b.key));
                (done.len(), done)
            }
        }
    }

    /// Retire one session: snapshot its scrollback into the log, close or
    /// keep its workspace per `retain`, and hand back the `Finished` the
    /// daemon's bookkeeping expects.
    fn finish(&mut self, key: &str, reason: Reason) -> Finished {
        let close = match self.cfg.retain {
            Retain::Always => false,
            Retain::Never => true,
            // Keep the scene of an unaccounted end for attach; clear the
            // ones whose plan says what happened.
            Retain::OnFailure => !reason.is_failure(),
        };
        self.finish_closing(key, reason, close)
    }

    /// Retire without closing anything — the session goes on running, and
    /// the pool simply stops carrying it.
    fn retire(&mut self, key: &str, reason: Reason) -> Finished {
        self.finish_closing(key, reason, false)
    }

    fn finish_closing(&mut self, key: &str, reason: Reason, close: bool) -> Finished {
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
            reason,
            token: session.token,
        }
    }
}

/// Advance one session from one sighting. `Some(reason)` means it should be
/// retired.
///
/// The whole rule, in one place: a pane that works is alive, a pane that is
/// blocked holds its slot and says so once, and a pane that has settled is
/// judged by the plan behind it — never by what is on its screen.
fn observe(
    session: &mut Session,
    agent: &AgentInfo,
    disposition: Disposition,
    grace: Duration,
) -> Option<Reason> {
    match agent.agent_status.as_str() {
        "working" => {
            session.working();
            None
        }
        "blocked" => {
            session.idle_since = None;
            session.idle_streak = 0;
            session.blocked_streak += 1;
            if session.is_blocked() && !session.toasted {
                session.toasted = true;
                note(&format!(
                    "{} is blocked in herdr — attach to unblock it",
                    session.label
                ));
            }
            None
        }
        "idle" | "done" => {
            session.blocked_streak = 0;
            session.toasted = false;
            session.idle_streak += 1;
            let since = *session.idle_since.get_or_insert_with(Instant::now);
            if session.idle_streak < IDLE_STREAK {
                return None;
            }
            match disposition {
                // The plan says what happened. Nothing on the screen could
                // add to that, so the session is done.
                Disposition::Verdict => Some(Reason::Verdict),
                // Nothing to read a verdict from: settling is the end.
                Disposition::NoPlan if since.elapsed() >= grace => Some(Reason::Settled),
                // Claimed, unaccounted for, and nobody is typing. Give it
                // the grace, then hand the plan back.
                Disposition::Held if since.elapsed() >= grace => {
                    note(&format!(
                        "{}{} settled with its plan still claimed and no verdict — recycling it",
                        if session.adopted { "adopted " } else { "" },
                        session.label
                    ));
                    Some(Reason::Recycled)
                }
                _ => None,
            }
        }
        // `unknown`, or a state this pin has never heard of: not an
        // observation, so no counter moves. The idle clock is only ever
        // *reset* by working, so a pane that never resolves is still bounded
        // by the grace it had already started spending.
        _ => None,
    }
}

/// Retry a herdr call while it keeps refusing with one named code. Both
/// codes this guards are startup seams — a pane whose shell is still coming
/// up, an agent that is not promptable yet — and both clear within seconds.
fn retry_while<T>(
    code: &str,
    mut call: impl FnMut() -> super::client::Result<T>,
) -> super::client::Result<T> {
    let mut attempt = 0;
    loop {
        match call() {
            Err(e) if e.code() == Some(code) && attempt < PROMPT_RETRIES => {
                attempt += 1;
                std::thread::sleep(PROMPT_BACKOFF);
            }
            other => break other,
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

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        Session {
            workspace_id: "w1".into(),
            pane_id: "p1".into(),
            label: "act plans/x.md → org/founder".into(),
            log: String::new(),
            started: "2026-08-14T0000".into(),
            token: None,
            adopted: false,
            idle_since: None,
            idle_streak: 0,
            blocked_streak: 0,
            toasted: false,
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

    const NOW: Duration = Duration::ZERO;

    /// The core of decision 0061: the same settled pane means opposite
    /// things depending on what the plan says, and nothing else is consulted.
    #[test]
    fn a_settled_pane_is_judged_by_its_plan_and_nothing_else() {
        let mut done = session();
        assert_eq!(
            observe(&mut done, &sighting("idle"), Disposition::Verdict, NOW),
            None
        );
        assert_eq!(
            observe(&mut done, &sighting("idle"), Disposition::Verdict, NOW),
            Some(Reason::Verdict)
        );

        let mut stalled = session();
        observe(&mut stalled, &sighting("idle"), Disposition::Held, NOW);
        assert_eq!(
            observe(&mut stalled, &sighting("idle"), Disposition::Held, NOW),
            Some(Reason::Recycled)
        );
    }

    /// One settled sighting is never enough: a dialog rendering mid-turn
    /// reads idle for a beat, and retiring on that beat closed a workspace
    /// over a waiting session (observed live, decision 0052).
    #[test]
    fn one_idle_sighting_is_not_a_settle() {
        let mut s = session();
        assert_eq!(
            observe(&mut s, &sighting("idle"), Disposition::Verdict, NOW),
            None
        );
        // …and work resumed in the gap puts the count back.
        observe(&mut s, &sighting("working"), Disposition::Held, NOW);
        assert_eq!(s.idle_streak, 0);
        assert!(s.idle_since.is_none());
        assert_eq!(
            observe(&mut s, &sighting("idle"), Disposition::Verdict, NOW),
            None
        );
    }

    /// A stall is graced; a verdict is not. The grace is there for work the
    /// pane cannot show, and a plan that already says what happened needs no
    /// such benefit of the doubt.
    #[test]
    fn the_grace_holds_a_stall_but_never_delays_a_verdict() {
        let grace = Duration::from_secs(3600);
        let mut stalled = session();
        for _ in 0..5 {
            assert_eq!(
                observe(&mut stalled, &sighting("idle"), Disposition::Held, grace),
                None,
                "still inside its grace"
            );
        }
        let mut settled = session();
        observe(&mut settled, &sighting("idle"), Disposition::Verdict, grace);
        assert_eq!(
            observe(&mut settled, &sighting("idle"), Disposition::Verdict, grace),
            Some(Reason::Verdict)
        );
    }

    /// A ritual has no plan to read, so settling is all it has — but it is
    /// still graced, because a dropped prompt looks the same as a fast run.
    #[test]
    fn a_ritual_settles_on_its_own_since_there_is_no_plan_to_read() {
        let mut s = session();
        observe(&mut s, &sighting("idle"), Disposition::NoPlan, NOW);
        assert_eq!(
            observe(&mut s, &sighting("idle"), Disposition::NoPlan, NOW),
            Some(Reason::Settled)
        );
    }

    #[test]
    fn blocked_is_believed_twice_toasts_once_and_holds_its_slot() {
        let mut s = session();
        assert_eq!(
            observe(&mut s, &sighting("blocked"), Disposition::Held, NOW),
            None
        );
        assert!(!s.is_blocked(), "one sighting is a flap, not a block");
        assert_eq!(
            observe(&mut s, &sighting("blocked"), Disposition::Held, NOW),
            None
        );
        assert!(s.is_blocked());
        assert!(s.toasted);
        // Still blocked, still holding, and the operator is not told twice.
        observe(&mut s, &sighting("blocked"), Disposition::Held, NOW);
        assert!(s.toasted);
        // Unblocked by hand: the slot is working again and can toast afresh.
        observe(&mut s, &sighting("working"), Disposition::Held, NOW);
        assert!(!s.is_blocked());
        assert!(!s.toasted);
    }

    /// A status this pin has never heard of is not an observation — but it
    /// does not reset a grace already being spent, so a pane that goes
    /// permanently unreadable is still recovered.
    #[test]
    fn an_unknown_status_moves_nothing_but_stops_no_clock() {
        let mut s = session();
        observe(&mut s, &sighting("idle"), Disposition::Held, NOW);
        let since = s.idle_since;
        assert_eq!(
            observe(&mut s, &sighting("unknown"), Disposition::Held, NOW),
            None
        );
        assert_eq!(s.idle_streak, 1, "unknown is not a settled sighting");
        assert_eq!(s.idle_since, since, "…nor a reason to forget one");
        assert_eq!(
            observe(&mut s, &sighting("idle"), Disposition::Held, NOW),
            Some(Reason::Recycled)
        );
    }

    #[test]
    fn an_agent_name_is_herdr_legal_however_ugly_the_key() {
        assert_eq!(agent_name("plan:plans/x.md"), "t-plan-plans-x-md");
        assert!(agent_name(&"plan:plans/".repeat(20)).len() <= 32);
    }
}
