//! What the runtime remembers across restarts, split by process (decision
//! 0046): the dispatcher's file carries plan runs, attempt stamps, and the
//! escalation records already announced; the rituals file carries each
//! ritual's last firing. Separate files because the processes are separate —
//! two writers on one JSON document is a race, two documents is not.
//!
//! Written under `.trellis/runtime/` — gitignored, single-machine, ephemeral
//! by construction. Losing either is safe rather than merely tolerable: an
//! empty rituals state fires each cadence once, and dispatch is guarded by
//! the taker's `ready → active` flip on disk, not by anything remembered
//! here. Persisted eagerly after every mutation, so a `kill -9` loses
//! nothing. A legacy combined `state.json` is read once, filtered by each
//! file's own keyspace, and left in place.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::config::RuntimeConfig;
use crate::dates::Date;

pub const DISPATCH_FILE: &str = "dispatch-state.json";
pub const RITUALS_FILE: &str = "rituals-state.json";
const LEGACY_FILE: &str = "state.json";

/// One task's run history. Only the last run is kept — the durable trail is
/// git and the session logs, never this file.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Run {
    /// ISO date the last session was spawned.
    pub last_fired: Option<String>,
    /// Unix seconds of the last spawn — what the dispatcher's retry cooldown
    /// reads, since a tight loop needs finer than a date.
    pub last_attempt_secs: Option<u64>,
    /// How that session ended, once it has been concluded — the `Reason`
    /// word (decision 0061). There is no exit code to record: a session is a
    /// pane, and what became of the *work* is in the plan, not in a number.
    pub last_outcome: Option<String>,
    /// Log path, relative to the runtime directory.
    pub last_log: Option<String>,
    /// Which `[prompts]` errand that session ran. What a finished session is
    /// owed depends on what it was sent to do: only `act` was sent to advance
    /// the plan, so only `act` returns an abandoned one to the queue. Absent
    /// in state written before this field existed — read as "not act".
    pub last_errand: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub version: u32,
    /// Keyed by [`key_ritual`] / [`key_plan`].
    pub runs: BTreeMap<String, Run>,
    /// Escalation records already seen open, so a channel adapter announces
    /// each one once. Keyed artifact#raised#by — stable when lines move.
    /// Dispatch-state only; empty and unused in the rituals file.
    pub seen_escalations: BTreeSet<String>,
    /// Whether the record set above has been observed at least once. A
    /// daemon meeting a root for the first time adopts its open records
    /// silently: they are the standing backlog, not news.
    pub escalations_seeded: bool,
    /// Which file this state loads from and saves to.
    #[serde(skip)]
    file: &'static str,
}

impl Default for State {
    fn default() -> Self {
        State {
            version: 2,
            runs: BTreeMap::new(),
            seen_escalations: BTreeSet::new(),
            escalations_seeded: false,
            file: DISPATCH_FILE,
        }
    }
}

pub fn key_ritual(name: &str) -> String {
    format!("ritual:{name}")
}

pub fn key_plan(rel: &str) -> String {
    format!("plan:{rel}")
}

impl State {
    fn path_for(root: &Path, file: &str) -> PathBuf {
        RuntimeConfig::runtime_dir(root).join(file)
    }

    /// The dispatcher's state: `plan:` runs plus the escalation ledger.
    pub fn load_dispatch(root: &Path) -> State {
        Self::load_file(root, DISPATCH_FILE, "plan:", true)
    }

    /// The rituals state: `ritual:` runs.
    pub fn load_rituals(root: &Path) -> State {
        Self::load_file(root, RITUALS_FILE, "ritual:", false)
    }

    /// Missing or unreadable state is empty state — never a startup failure.
    /// A missing split file falls back to the pre-0046 combined file,
    /// filtered to this file's keyspace.
    fn load_file(root: &Path, file: &'static str, prefix: &str, escalations: bool) -> State {
        let mut state: State = std::fs::read_to_string(Self::path_for(root, file))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_else(|| {
                let legacy: Option<State> =
                    std::fs::read_to_string(Self::path_for(root, LEGACY_FILE))
                        .ok()
                        .and_then(|t| serde_json::from_str(&t).ok());
                match legacy {
                    Some(mut legacy) => {
                        legacy.runs.retain(|k, _| k.starts_with(prefix));
                        if !escalations {
                            legacy.seen_escalations.clear();
                            legacy.escalations_seeded = false;
                        }
                        legacy
                    }
                    None => State::default(),
                }
            });
        state.file = file;
        state.version = 2;
        state
    }

    pub fn save(&self, root: &Path) -> anyhow::Result<()> {
        let dir = RuntimeConfig::runtime_dir(root);
        std::fs::create_dir_all(&dir)?;
        let rendered = serde_json::to_string_pretty(self)?;
        let tmp = tempfile::NamedTempFile::new_in(&dir)?;
        std::fs::write(tmp.path(), format!("{rendered}\n"))?;
        tmp.persist(Self::path_for(root, self.file))
            .map_err(|e| anyhow::anyhow!("atomic rename failed: {e}"))?;
        Ok(())
    }

    pub fn last_fired(&self, key: &str) -> Option<Date> {
        self.runs
            .get(key)
            .and_then(|r| r.last_fired.as_deref())
            .and_then(Date::parse)
    }

    pub fn last_attempt_secs(&self, key: &str) -> Option<u64> {
        self.runs.get(key).and_then(|r| r.last_attempt_secs)
    }

    /// Record a spawn. The fire date is what the rituals scheduler reads
    /// back and the attempt stamp is what the dispatch cooldown reads, so
    /// both are set on a successful spawn and never on a skipped one.
    pub fn fired(&mut self, key: &str, errand: &str, today: Date, log: Option<String>) {
        let run = self.runs.entry(key.to_string()).or_default();
        run.last_fired = Some(today.to_string());
        run.last_attempt_secs = Some(now_secs());
        run.last_outcome = None;
        run.last_log = log;
        run.last_errand = Some(errand.to_string());
    }

    /// Attach the log name to an attempt already recorded. `fired` is
    /// written *before* the spawn, so that a crash mid-spawn still leaves
    /// the trail startup recovery reads; the log name only exists after.
    pub fn set_log(&mut self, key: &str, log: String) {
        self.runs.entry(key.to_string()).or_default().last_log = Some(log);
    }

    /// What the last session on this key was sent to do.
    pub fn last_errand(&self, key: &str) -> Option<&str> {
        self.runs.get(key).and_then(|r| r.last_errand.as_deref())
    }

    /// Record how a session ended.
    ///
    /// An end that leaves the plan unaccounted for also re-stamps the
    /// attempt clock. The cooldown exists to pace *retries*, and the retry
    /// begins when the plan comes back to the queue — not when the session
    /// that stalled was first spawned, which may have been hours earlier.
    /// Without this a recycled session is re-dispatched by the very next
    /// pass, cooldown or no cooldown.
    pub fn concluded(&mut self, key: &str, reason: super::session::Reason) {
        let run = self.runs.entry(key.to_string()).or_default();
        run.last_outcome = Some(reason.as_str().to_string());
        if reason.is_failure() {
            run.last_attempt_secs = Some(now_secs());
        }
    }
}

/// Wall-clock unix seconds. The cooldown compares two of these, so the only
/// property that matters is monotonicity-in-practice between ticks.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_its_own_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let today = Date::parse("2026-08-03").unwrap();
        let mut state = State::load_rituals(dir.path());
        state.fired(
            &key_ritual("conventions lint"),
            "ritual",
            today,
            Some("a.log".into()),
        );
        state.concluded(
            &key_ritual("conventions lint"),
            super::super::session::Reason::Verdict,
        );
        state.save(dir.path()).unwrap();

        let read = State::load_rituals(dir.path());
        assert_eq!(
            read.last_fired(&key_ritual("conventions lint")),
            Some(today)
        );
        assert_eq!(
            read.runs[&key_ritual("conventions lint")]
                .last_outcome
                .as_deref(),
            Some("verdict")
        );
        assert!(read.last_attempt_secs(&key_ritual("conventions lint")).is_some());
    }

    #[test]
    fn a_legacy_combined_file_is_read_split_by_keyspace() {
        let dir = tempfile::TempDir::new().unwrap();
        let runtime = RuntimeConfig::runtime_dir(dir.path());
        std::fs::create_dir_all(&runtime).unwrap();
        std::fs::write(
            runtime.join(LEGACY_FILE),
            r#"{"version":1,"runs":{
                "plan:plans/a.md":{"last_fired":"2026-08-01"},
                "ritual:conventions lint":{"last_fired":"2026-08-02"},
                "dispatch":{"last_fired":"2026-08-03"}},
                "seen_escalations":["plans/a.md#d#r"],"escalations_seeded":true}"#,
        )
        .unwrap();

        let dispatch = State::load_dispatch(dir.path());
        assert!(dispatch.runs.contains_key("plan:plans/a.md"));
        assert!(!dispatch.runs.contains_key("ritual:conventions lint"));
        assert!(
            !dispatch.runs.contains_key("dispatch"),
            "the scan's own calendar key dies with the calendar gate"
        );
        assert!(dispatch.escalations_seeded);

        let rituals = State::load_rituals(dir.path());
        assert!(rituals.runs.contains_key("ritual:conventions lint"));
        assert!(!rituals.runs.contains_key("plan:plans/a.md"));
        assert!(rituals.seen_escalations.is_empty());
    }

    #[test]
    fn corrupt_state_reads_as_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = State::path_for(dir.path(), DISPATCH_FILE);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{not json").unwrap();
        assert!(State::load_dispatch(dir.path()).runs.is_empty());
    }

    #[test]
    fn an_unfired_key_has_no_date() {
        assert_eq!(
            State::load_dispatch(tempfile::TempDir::new().unwrap().path())
                .last_fired(&key_plan("plans/x.md")),
            None
        );
    }
}
