//! What the daemon remembers across restarts: when each ritual last fired,
//! when each plan was last dispatched, and which escalation records it has
//! already seen open.
//!
//! Written to `.trellis/runtime/state.json` — gitignored, single-machine,
//! ephemeral by construction. Losing it is safe rather than merely tolerable:
//! an empty state fires each cadence once, and dispatch is guarded by the
//! taker's `ready → active` flip on disk, not by anything remembered here.
//! Persisted eagerly after every mutation, so a `kill -9` loses nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::config::RuntimeConfig;
use crate::dates::Date;

pub const FILE: &str = "state.json";

/// One task's run history. Only the last run is kept — the durable trail is
/// git and the session logs, never this file.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Run {
    /// ISO date the last session was spawned.
    pub last_fired: Option<String>,
    /// Exit code of that session, once it has been reaped.
    pub last_exit: Option<i32>,
    /// Log path, relative to the runtime directory.
    pub last_log: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub version: u32,
    /// Keyed by [`key_ritual`] / [`key_plan`] / [`DISPATCH`].
    pub runs: BTreeMap<String, Run>,
    /// Escalation records already seen open, so a channel adapter announces
    /// each one once. Keyed artifact#raised#by — stable when lines move.
    pub seen_escalations: BTreeSet<String>,
    /// Whether the record set above has been observed at least once. A
    /// daemon meeting a root for the first time adopts its open records
    /// silently: they are the standing backlog, not news.
    pub escalations_seeded: bool,
}

impl Default for State {
    fn default() -> Self {
        State {
            version: 1,
            runs: BTreeMap::new(),
            seen_escalations: BTreeSet::new(),
            escalations_seeded: false,
        }
    }
}

/// The scan itself, as distinct from the plans it dispatches.
pub const DISPATCH: &str = "dispatch";

pub fn key_ritual(name: &str) -> String {
    format!("ritual:{name}")
}

pub fn key_plan(rel: &str) -> String {
    format!("plan:{rel}")
}

impl State {
    pub fn path(root: &Path) -> PathBuf {
        RuntimeConfig::runtime_dir(root).join(FILE)
    }

    /// Missing or unreadable state is empty state — never a startup failure.
    pub fn load(root: &Path) -> State {
        std::fs::read_to_string(State::path(root))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, root: &Path) -> anyhow::Result<()> {
        let dir = RuntimeConfig::runtime_dir(root);
        std::fs::create_dir_all(&dir)?;
        let rendered = serde_json::to_string_pretty(self)?;
        let tmp = tempfile::NamedTempFile::new_in(&dir)?;
        std::fs::write(tmp.path(), format!("{rendered}\n"))?;
        tmp.persist(State::path(root))
            .map_err(|e| anyhow::anyhow!("atomic rename failed: {e}"))?;
        Ok(())
    }

    pub fn last_fired(&self, key: &str) -> Option<Date> {
        self.runs
            .get(key)
            .and_then(|r| r.last_fired.as_deref())
            .and_then(Date::parse)
    }

    /// Record a spawn. The fire date is what the scheduler reads back, so it
    /// is set on a successful spawn and never on a skipped one.
    pub fn fired(&mut self, key: &str, today: Date, log: Option<String>) {
        let run = self.runs.entry(key.to_string()).or_default();
        run.last_fired = Some(today.to_string());
        run.last_exit = None;
        run.last_log = log;
    }

    pub fn finished(&mut self, key: &str, exit: Option<i32>) {
        self.runs.entry(key.to_string()).or_default().last_exit = Some(exit.unwrap_or(-1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_the_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let today = Date::parse("2026-08-03").unwrap();
        let mut state = State::default();
        state.fired(&key_ritual("conventions lint"), today, Some("a.log".into()));
        state.finished(&key_ritual("conventions lint"), Some(0));
        state
            .seen_escalations
            .insert("plans/x.md#2026-07-30#org/coder".into());
        state.save(dir.path()).unwrap();

        let read = State::load(dir.path());
        assert_eq!(
            read.last_fired(&key_ritual("conventions lint")),
            Some(today)
        );
        assert_eq!(
            read.runs[&key_ritual("conventions lint")].last_exit,
            Some(0)
        );
        assert!(read
            .seen_escalations
            .contains("plans/x.md#2026-07-30#org/coder"));
    }

    #[test]
    fn corrupt_state_reads_as_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = State::path(dir.path());
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{not json").unwrap();
        assert!(State::load(dir.path()).runs.is_empty());
    }

    #[test]
    fn an_unfired_key_has_no_date() {
        assert_eq!(State::default().last_fired(&key_plan("plans/x.md")), None);
    }
}
