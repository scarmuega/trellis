//! The escalation channel seam.
//!
//! Decision 0036 kept escalations in the tree and accepted the cost plainly:
//! a record notifies nobody. The fix it chartered was a push added *over* the
//! records, never instead of them — so this module watches for records that
//! newly read `status: open` and hands them to whatever adapters are
//! registered. The record stays the system of record; an adapter only ever
//! announces one.
//!
//! **No adapter ships in this slice** (decision 0038). The live channel is
//! the pull surface — `/api/escalations`, the board, the root itself — and a
//! configured-but-unimplemented transport is refused at startup rather than
//! silently dropped. What is here is the seam a webhook, a desktop
//! notification, or a chat adapter attaches to without touching the loop.

use super::state::State;
use crate::escalate::{self, ListedRecord};
use crate::tree::Tree;

/// A transport that announces newly opened escalation records.
pub trait Channel: Send {
    fn name(&self) -> &str;
    /// Announce one record. Errors are reported and never fatal: a channel
    /// that is down must not stop the domain's clock.
    fn escalation_opened(&self, record: &ListedRecord) -> anyhow::Result<()>;
}

#[derive(Default)]
pub struct Channels(Vec<Box<dyn Channel>>);

impl Channels {
    pub fn new() -> Channels {
        Channels(Vec::new())
    }

    pub fn register(&mut self, channel: Box<dyn Channel>) {
        self.0.push(channel);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Push each record to every adapter, collecting failures as messages
    /// for the daemon's own log.
    pub fn announce(&self, records: &[ListedRecord]) -> Vec<String> {
        let mut failures = Vec::new();
        for record in records {
            for channel in &self.0 {
                if let Err(e) = channel.escalation_opened(record) {
                    failures.push(format!(
                        "channel {} could not announce {}: {e}",
                        channel.name(),
                        record.artifact
                    ));
                }
            }
        }
        failures
    }
}

/// Identity of a record, stable across edits that move it: the artifact, the
/// date it was raised, and who raised it. Line numbers are not identity —
/// an edit above a record would re-announce it.
pub fn key(record: &ListedRecord) -> String {
    format!(
        "{}#{}#{}",
        record.artifact,
        record.raised.as_deref().unwrap_or("?"),
        record.by.as_deref().unwrap_or("?")
    )
}

/// Records that read `open` now and did not last time. Marks them seen, so a
/// record is announced once and not once per tick. The very first pass over a
/// root seeds silently: the records already there are its standing backlog.
pub fn detect_new(tree: &Tree, state: &mut State) -> Vec<ListedRecord> {
    let open = escalate::list(tree, false);
    let keys: Vec<String> = open.iter().map(key).collect();
    let current: std::collections::BTreeSet<String> = keys.iter().cloned().collect();

    if !state.escalations_seeded {
        state.seen_escalations = current;
        state.escalations_seeded = true;
        return Vec::new();
    }

    let fresh = open
        .into_iter()
        .zip(&keys)
        .filter(|(_, k)| !state.seen_escalations.contains(*k))
        .map(|(record, _)| record)
        .collect();
    // Tracking exactly what is open now also forgets resolved records, so a
    // record that reopens announces again — which is what a reopened blocker
    // is.
    state.seen_escalations = current;
    fresh
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::root::Root;

    /// The smallest thing `Root::discover` accepts, plus one plan.
    fn root_with(escalations: &str) -> (tempfile::TempDir, String) {
        let dir = tempfile::TempDir::new().unwrap();
        let p = dir.path();
        std::fs::write(p.join("conventions.md"), "# conventions\n").unwrap();
        for d in ["problem", "solution", "org", "plans"] {
            std::fs::create_dir_all(p.join(d)).unwrap();
        }
        std::fs::write(
            p.join("plans/ship.md"),
            format!("---\nowner: org/founder\nstatus: blocked\n---\n# ship\n{escalations}"),
        )
        .unwrap();
        (dir, "plans/ship.md".to_string())
    }

    fn load(dir: &tempfile::TempDir) -> Tree {
        Tree::load(Root {
            path: dir.path().to_path_buf(),
        })
        .unwrap()
    }

    const ONE: &str = "\n## Escalations\n\n```yaml\nraised: 2026-08-01\nby: org/founder\nto: org/founder\nstatus: open\nasks: unblock me\n```\n";

    #[test]
    fn the_first_pass_adopts_the_backlog_silently() {
        let (dir, _) = root_with(ONE);
        let mut state = State::default();
        assert!(detect_new(&load(&dir), &mut state).is_empty());
        assert!(state.escalations_seeded);
        assert_eq!(state.seen_escalations.len(), 1);
    }

    #[test]
    fn a_record_opened_after_seeding_is_announced_once() {
        let (dir, plan) = root_with("");
        let mut state = State::default();
        detect_new(&load(&dir), &mut state);

        let path = dir.path().join(&plan);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, format!("{text}{ONE}")).unwrap();

        let fresh = detect_new(&load(&dir), &mut state);
        assert_eq!(fresh.len(), 1);
        assert_eq!(fresh[0].asks.as_deref(), Some("unblock me"));
        assert!(
            detect_new(&load(&dir), &mut state).is_empty(),
            "announced once"
        );
    }

    #[test]
    fn resolving_a_record_stops_tracking_it() {
        let (dir, plan) = root_with(ONE);
        let mut state = State::default();
        detect_new(&load(&dir), &mut state);

        let path = dir.path().join(&plan);
        let text = std::fs::read_to_string(&path).unwrap();
        std::fs::write(&path, text.replace("status: open", "status: resolved")).unwrap();

        assert!(detect_new(&load(&dir), &mut state).is_empty());
        assert!(state.seen_escalations.is_empty());
    }
}
