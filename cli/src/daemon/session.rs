//! What a session *is* to the daemon's bookkeeping, independent of the pane
//! it runs in.
//!
//! One session per due task, carried by herdr, with a log under
//! `.trellis/runtime/logs/` holding the header and — at retirement — the
//! pane's own scrollback. That log is the session-trail half of the ledger;
//! git holds the durable half.
//!
//! Two guards, at different scales. Within the process, a task already in
//! flight is never launched twice. Across processes and restarts, nothing
//! here is the guard: dispatch idempotence comes from the plan going
//! `ready → active` on disk before its session starts (decision 0029, moved
//! from the taker to the runtime by decision 0053).
//!
//! There is deliberately no exit code here. A session ends when its *plan*
//! reaches a verdict on disk (decision 0061) — the pane settling is a
//! symptom, not the event — so what a retirement reports is a `Reason`, and
//! the truth about the work stays in the artifact its taker wrote.

use std::path::Path;

use serde::Serialize;

use crate::dates::Date;

/// One in-flight session, as `/api/status` reports it. Deserialize too: the
/// read-only server reads these back out of the dispatcher's snapshot file.
#[derive(Debug, Clone, Default, Serialize, serde::Deserialize)]
#[serde(default)]
pub struct InFlightView {
    pub key: String,
    pub label: String,
    pub log: String,
    pub started: String,
    pub token: Option<String>,
    /// The herdr pane this session runs in — the address an operator
    /// attaches to. Optional only for the snapshot's sake: a view read back
    /// from an older `status.json` may not carry one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pane: Option<String>,
    /// What this session declared it is waiting on, while its lease is live.
    /// A held slot the operator cannot account for is the complaint decision
    /// 0052 raised against the old `Waiting` phase, so the wait says itself.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiting: Option<String>,
}

/// Why a session left the fleet. This is the whole of what retirement
/// reports — it drives the log line and whether the workspace is kept for
/// an operator to attach to, and nothing else branches on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Its plan reached a verdict on disk: retired, blocked, passed on, or
    /// parked behind a declared `handoff:`. The ordinary end.
    Verdict,
    /// A ritual session settled at its prompt. Rituals leave no artifact the
    /// runtime can read a verdict from, so settling is all there is.
    Settled,
    /// It went idle holding a plan it never gave a verdict for. The plan is
    /// handed back to `ready` and the retry cooldown paces the next attempt.
    Recycled,
    /// Herdr no longer knows the pane — closed out from under the daemon.
    Gone,
    /// The herdr server went away for long enough that its panes must be
    /// presumed gone with it.
    ServerLost,
    /// Left running on purpose: blocked at its own UI when a drain had
    /// nobody to unblock it. Its marker line stays, so the next daemon
    /// adopts it rather than judging it abandoned.
    Detached,
    /// Closed by the daemon's own shutdown.
    Stopped,
}

impl Reason {
    /// The word that goes in `state.json` and in the log line.
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::Verdict => "verdict",
            Reason::Settled => "settled",
            Reason::Recycled => "recycled",
            Reason::Gone => "gone",
            Reason::ServerLost => "server-lost",
            Reason::Detached => "detached",
            Reason::Stopped => "stopped",
        }
    }

    /// Whether the session ended without the plan being accounted for.
    pub fn is_failure(self) -> bool {
        matches!(self, Reason::Recycled | Reason::Gone | Reason::ServerLost)
    }

    /// Whether the workspace must come down whatever `retain` says.
    ///
    /// Only a recycle, and it is not a retention preference — it is the other
    /// half of handing the plan back. A recycled session is one the runtime
    /// has just judged nobody to be on: its claim is released and the plan
    /// requeued, so within one cooldown a *second* session is dispatched to
    /// the same plan. Leaving the first pane running makes two agents writing
    /// to one artifact, which is the exact strand `active` means somebody is
    /// on it exists to prevent (decision 0052), arriving by another road.
    /// The scrollback is copied into the session log before the close either
    /// way, so this costs live attach, not evidence.
    pub fn must_close(self) -> bool {
        matches!(self, Reason::Recycled)
    }
}

#[derive(Debug)]
pub struct Finished {
    pub key: String,
    pub label: String,
    pub reason: Reason,
    pub token: Option<String>,
}

/// `YYYY-MM-DDThhmmss` — the date honours `TRELLIS_TODAY`, the time is the
/// wall clock, so log names sort chronologically and stay legible.
pub fn timestamp(today: Date) -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let sod = secs.rem_euclid(86_400);
    format!(
        "{today}T{:02}{:02}{:02}",
        sod / 3600,
        (sod % 3600) / 60,
        sod % 60
    )
}

/// Filesystem-safe form of a task key (`plan:plans/x.md` → `plan-plans-x.md`).
/// The kernel names the same shape for wait leases, and one spelling of it is
/// enough — the daemon may depend on the kernel, never the other way round.
pub use crate::waits::slug;

/// A copy-pasteable rendering of the argv for the log header. Never parsed —
/// the daemon runs the array, not this string.
pub fn shell_quote(argv: &[String]) -> String {
    argv.iter()
        .map(|a| {
            if a.is_empty()
                || a.contains(|c: char| c.is_whitespace() || "\"'$`\\|&;<>()*?[]#~".contains(c))
            {
                format!("'{}'", a.replace('\'', r"'\''"))
            } else {
                a.clone()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// Append a pane's scrollback to its session log at retirement — the only
/// thing that ever writes a session's own output, since the session ran in a
/// pane rather than behind a pipe.
pub fn append_tail(path: &Path, tail: &str) -> std::io::Result<()> {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new().append(true).open(path)?;
    writeln!(file, "\n# --- pane scrollback at retirement ---")?;
    writeln!(file, "{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_names_are_filesystem_safe() {
        assert_eq!(slug("plan:plans/ship-it.md"), "plan-plans-ship-it.md");
        assert_eq!(slug("ritual:conventions lint"), "ritual-conventions-lint");
    }

    #[test]
    fn the_log_header_quotes_what_a_shell_would_need_quoted() {
        assert_eq!(shell_quote(&["claude".into(), "-p".into()]), "claude -p");
        assert_eq!(
            shell_quote(&["advance plans/x.md".into()]),
            "'advance plans/x.md'"
        );
        assert_eq!(shell_quote(&["it's".into()]), r"'it'\''s'");
    }

    #[test]
    fn only_an_unaccounted_end_keeps_a_workspace_open() {
        assert!(Reason::Recycled.is_failure());
        assert!(Reason::Gone.is_failure());
        assert!(Reason::ServerLost.is_failure());
        assert!(!Reason::Verdict.is_failure());
        assert!(!Reason::Settled.is_failure());
        assert!(!Reason::Stopped.is_failure());
    }
}
