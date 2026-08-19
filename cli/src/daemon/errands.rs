//! The operator's queue: asks that have been made and not yet answered.
//!
//! An errand is the one thing the runtime asks a human to author by hand, and
//! it was the one thing the runtime kept only in memory — a `Vec` in the
//! dispatch process, drained with `mem::take`. Every way a queued ask failed to
//! become a session handed the typing back to the operator: no free slot, a
//! session already on the plan, a dispatcher that restarted. The instruction
//! was gone and nothing said so.
//!
//! So the queue is a file, and the rule it keeps is simple: **an errand is
//! never refused for timing and never displaced.** A busy plan queues, a full
//! fleet waits, a restart is survived, and a second ask joins the first rather
//! than replacing it. The only thing that stops an ask firing is the plan
//! changing underneath it — and even then it is held, marked stale, and shown
//! to its author to confirm or discard.
//!
//! One writer, by construction: `shared.dispatches` is true only in the
//! dispatch process and `serve` relays to it, so this document has the single
//! owner `state.rs` requires of a JSON file. Saved eagerly after every
//! mutation, atomically renamed, so a `kill -9` loses nothing.
//!
//! Unlike `dispatch-state.json`, losing this file is **not** safe: it is the
//! only record that an ask was made. That is the one place this module departs
//! from its sibling's posture, and it is deliberate — the alternative is what
//! sent the operator back to the keyboard.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::config::RuntimeConfig;

pub const ERRANDS_FILE: &str = "errands.json";

/// Where one ask has got to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum EntryState {
    /// Waiting for a slot and a free plan. The ordinary state.
    Pending,
    /// The plan changed under it, so it is no longer certainly an ask about
    /// this plan. Held, never fired, and shown to its author.
    Stale,
    /// A session was started for it. Removed at that session's conclusion, or
    /// returned to `Pending` by `reconcile` if the daemon died in between.
    Dispatched { key: String },
}

/// One ask, as made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    pub id: u64,
    /// The plan rel ("plans/x.md").
    pub plan: String,
    /// The whole of the ask, written at the moment it was wanted.
    pub instruction: String,
    /// Sizing chosen at the call; `None` falls back to the plan's tier.
    pub model: Option<String>,
    pub effort: Option<String>,
    /// Unix seconds, so the board can say how long it has been waiting.
    pub asked_at: u64,
    /// What the plan said when the ask was written (`plan_ops::fingerprint`).
    pub fingerprint: u64,
    #[serde(flatten)]
    pub state: EntryState,
}

impl Entry {
    pub fn is_pending(&self) -> bool {
        self.state == EntryState::Pending
    }
}

/// The queue, in the order the asks were made.
#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Queue {
    pub version: u32,
    /// Monotonic, persisted — an id has to survive a restart, and a counter
    /// costs nothing where a random source would be a dependency.
    next_id: u64,
    pub entries: Vec<Entry>,
}

fn path_for(root: &Path) -> PathBuf {
    RuntimeConfig::runtime_dir(root).join(ERRANDS_FILE)
}

impl Queue {
    /// Missing or unreadable is an empty queue rather than a startup failure —
    /// the same posture as the state files. It is the one loss this module
    /// cannot make good, and it is reported by the caller rather than hidden.
    pub fn load(root: &Path) -> Queue {
        let mut queue: Queue = std::fs::read_to_string(path_for(root))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default();
        queue.version = 1;
        // A file hand-edited into inconsistency should not mint a duplicate id.
        let highest = queue.entries.iter().map(|e| e.id).max().unwrap_or(0);
        queue.next_id = queue.next_id.max(highest + 1);
        queue
    }

    pub fn save(&self, root: &Path) -> anyhow::Result<()> {
        let dir = RuntimeConfig::runtime_dir(root);
        std::fs::create_dir_all(&dir)?;
        let rendered = serde_json::to_string_pretty(self)?;
        let tmp = tempfile::NamedTempFile::new_in(&dir)?;
        std::fs::write(tmp.path(), format!("{rendered}\n"))?;
        tmp.persist(path_for(root))
            .map_err(|e| anyhow::anyhow!("atomic rename failed: {e}"))?;
        Ok(())
    }

    /// Queue an ask, returning its entry.
    ///
    /// A plan may carry several: they fire in the order they were made, one at
    /// a time, because `fire` defers a key already in flight. Nothing is ever
    /// displaced — the operator asking twice meant to ask twice.
    ///
    /// The one thing that does not queue twice is the *same* instruction
    /// already pending on the same plan. That is recognition rather than
    /// displacement: a double-click is one ask, and treating it as two would
    /// build a backlog nobody asked for.
    pub fn push(
        &mut self,
        plan: &str,
        instruction: &str,
        model: Option<String>,
        effort: Option<String>,
        asked_at: u64,
        fingerprint: u64,
    ) -> Entry {
        let instruction = instruction.trim().to_string();
        if let Some(existing) = self
            .entries
            .iter()
            .find(|e| e.is_pending() && e.plan == plan && e.instruction == instruction)
        {
            return existing.clone();
        }
        let entry = Entry {
            id: self.next_id,
            plan: plan.to_string(),
            instruction,
            model,
            effort,
            asked_at,
            fingerprint,
            state: EntryState::Pending,
        };
        self.next_id += 1;
        self.entries.push(entry.clone());
        entry
    }

    pub fn get(&self, id: u64) -> Option<&Entry> {
        self.entries.iter().find(|e| e.id == id)
    }

    pub fn remove(&mut self, id: u64) -> Option<Entry> {
        let at = self.entries.iter().position(|e| e.id == id)?;
        Some(self.entries.remove(at))
    }

    /// Drop the entry a concluded session was started for. Keyed by task key
    /// rather than id because that is what the session's end carries.
    pub fn remove_dispatched(&mut self, key: &str) -> Option<Entry> {
        let at = self
            .entries
            .iter()
            .position(|e| e.state == EntryState::Dispatched { key: key.to_string() })?;
        Some(self.entries.remove(at))
    }

    fn set_state(&mut self, id: u64, state: EntryState) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.state = state;
        }
    }

    pub fn mark_dispatched(&mut self, id: u64, key: &str) {
        self.set_state(id, EntryState::Dispatched { key: key.to_string() });
    }

    /// The plan no longer says what it said when this was written.
    pub fn mark_stale(&mut self, id: u64) {
        self.set_state(id, EntryState::Stale);
    }

    /// The operator looked at a stale ask and said it still stands. Re-pinned
    /// to what the plan says now, so it is judged against the plan its author
    /// has just read rather than the one they wrote it against.
    pub fn confirm(&mut self, id: u64, fingerprint: u64) -> bool {
        let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) else {
            return false;
        };
        entry.fingerprint = fingerprint;
        entry.state = EntryState::Pending;
        true
    }

    /// Pending asks, oldest first — the order the drain fires them in.
    pub fn pending(&self) -> Vec<&Entry> {
        self.entries.iter().filter(|e| e.is_pending()).collect()
    }

    pub fn has_pending(&self) -> bool {
        self.entries.iter().any(|e| e.is_pending())
    }

    /// Every ask still owed on one plan, whatever its state.
    pub fn for_plan(&self, plan: &str) -> Vec<&Entry> {
        self.entries.iter().filter(|e| e.plan == plan).collect()
    }

    /// Startup repair, the errand half of `marker::reconcile`: an ask marked
    /// dispatched whose session is not in flight belongs back in the queue.
    /// The daemon died between the spawn and the conclusion, and the operator
    /// is owed the ask, not an explanation. Returns what was requeued.
    pub fn reconcile(&mut self, live: &[String]) -> Vec<u64> {
        let mut requeued = Vec::new();
        for entry in self.entries.iter_mut() {
            if let EntryState::Dispatched { key } = &entry.state {
                if !live.contains(key) {
                    requeued.push(entry.id);
                    entry.state = EntryState::Pending;
                }
            }
        }
        requeued
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn queue() -> Queue {
        Queue::load(Path::new("/nonexistent-root"))
    }

    fn push(q: &mut Queue, plan: &str, instruction: &str) -> u64 {
        q.push(plan, instruction, None, None, 1_000, 42).id
    }

    /// The whole point: two asks on one plan are two asks.
    #[test]
    fn a_second_ask_joins_the_first_rather_than_replacing_it() {
        let mut q = queue();
        let a = push(&mut q, "plans/x.md", "tighten the scope");
        let b = push(&mut q, "plans/x.md", "actually, split it");
        assert_ne!(a, b);
        assert_eq!(
            q.pending().iter().map(|e| e.id).collect::<Vec<_>>(),
            vec![a, b],
            "and they fire in the order they were made"
        );
    }

    /// But a double-click is one ask.
    #[test]
    fn the_same_instruction_twice_is_recognised_not_duplicated() {
        let mut q = queue();
        let a = push(&mut q, "plans/x.md", "tighten the scope");
        let again = push(&mut q, "plans/x.md", "  tighten the scope  ");
        assert_eq!(a, again);
        assert_eq!(q.pending().len(), 1);
        // Once it is running, the same words are a new ask: the operator is
        // asking for it again, having seen the first one taken.
        q.mark_dispatched(a, "plan:plans/x.md");
        let third = push(&mut q, "plans/x.md", "tighten the scope");
        assert_ne!(a, third);
    }

    #[test]
    fn it_round_trips_through_its_own_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let mut q = Queue::load(dir.path());
        let a = push(&mut q, "plans/x.md", "tighten the scope");
        let b = push(&mut q, "plans/y.md", "check the readout");
        q.mark_dispatched(b, "plan:plans/y.md");
        q.save(dir.path()).unwrap();

        let read = Queue::load(dir.path());
        assert_eq!(read.entries.len(), 2);
        assert_eq!(read.pending().len(), 1);
        assert_eq!(read.get(a).unwrap().instruction, "tighten the scope");
        assert_eq!(
            read.get(b).unwrap().state,
            EntryState::Dispatched {
                key: "plan:plans/y.md".into()
            }
        );
        // And a fresh push does not reuse an id read back from disk.
        let mut read = read;
        assert!(push(&mut read, "plans/z.md", "one more") > b);
    }

    /// A daemon that died between the spawn and the conclusion owes the
    /// operator the ask, not an explanation.
    #[test]
    fn a_dispatched_ask_whose_session_is_gone_goes_back_in_the_queue() {
        let mut q = queue();
        let lost = push(&mut q, "plans/x.md", "tighten the scope");
        let live = push(&mut q, "plans/y.md", "check the readout");
        q.mark_dispatched(lost, "plan:plans/x.md");
        q.mark_dispatched(live, "plan:plans/y.md");

        let requeued = q.reconcile(&["plan:plans/y.md".to_string()]);
        assert_eq!(requeued, vec![lost]);
        assert!(q.get(lost).unwrap().is_pending());
        assert!(
            !q.get(live).unwrap().is_pending(),
            "a session still running keeps its ask"
        );
    }

    /// A stale ask is held, not dropped, and confirming re-pins it to the plan
    /// its author has just read.
    #[test]
    fn a_stale_ask_waits_for_its_author() {
        let mut q = queue();
        let id = push(&mut q, "plans/x.md", "tighten the scope");
        q.mark_stale(id);
        assert!(q.pending().is_empty(), "it does not fire");
        assert_eq!(q.for_plan("plans/x.md").len(), 1, "and it does not vanish");

        assert!(q.confirm(id, 99));
        assert!(q.get(id).unwrap().is_pending());
        assert_eq!(q.get(id).unwrap().fingerprint, 99);
    }

    #[test]
    fn a_concluded_session_takes_its_ask_with_it() {
        let mut q = queue();
        let id = push(&mut q, "plans/x.md", "tighten the scope");
        q.mark_dispatched(id, "plan:plans/x.md");
        assert_eq!(q.remove_dispatched("plan:plans/x.md").unwrap().id, id);
        assert!(q.entries.is_empty());
        assert!(q.remove_dispatched("plan:plans/x.md").is_none());
    }
}
