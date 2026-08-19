//! A declared wait: the session's own statement that it is minding work
//! which outlives its turn, and when to stop believing it.
//!
//! The runtime reads completion from the plan (decision 0061), and a pane
//! that has gone quiet over a plan with no verdict is a stall — graced, then
//! recycled. That rule cannot tell a dead session from one waiting on a build
//! it started, because herdr reports screen state and both look identical on
//! screen. `handoff:` is the wrong answer for this case: it is a *verdict*,
//! parking the plan for someone else to move, and it closes the pane — which
//! would kill the very work being waited on.
//!
//! So the session says so, and the saying expires. A wait is a **lease**: a
//! liveness claim with a deadline, not a completion claim. Completion is
//! still only ever written to the plan.
//!
//! It lives beside the daemon's other ephemera rather than in the plan's
//! frontmatter, for the same reason `.trellis/acting-role` does: an expiry is
//! a fact about the *session*, and a wall-clock timestamp churning through a
//! git-tracked artifact is commit noise. Like that marker, this fails open —
//! an unwritable or unparseable lease is simply no lease, and the plan's own
//! status is what the runtime falls back to.
//!
//! One file per plan, written whole and replaced by rename, so the two
//! processes that touch it (the session writing, the daemon reading) need no
//! lock: there is no read-modify-write to serialize.

use std::path::{Path, PathBuf};

/// What a session declared, and when it said it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wait {
    /// When the lease was written — kept so a cap tightened after the fact
    /// still binds a lease taken under the old one.
    pub written_secs: u64,
    /// When the session asked to be believed until.
    pub expires_secs: u64,
    /// What it is waiting on, in its own words. For the operator, never
    /// parsed.
    pub what: String,
}

impl Wait {
    /// When this lease stops being believed: what the session asked for, or
    /// the operator's ceiling measured from the moment it asked — whichever
    /// comes first.
    pub fn deadline(&self, cap_secs: u64) -> u64 {
        self.expires_secs
            .min(self.written_secs.saturating_add(cap_secs))
    }

    /// Whether it is still believed at `now`.
    pub fn live(&self, now: u64, cap_secs: u64) -> bool {
        now < self.deadline(cap_secs)
    }
}

/// Where the leases live. Named here rather than taken from the daemon's
/// `RuntimeConfig::runtime_dir` because the kernel never depends on the
/// daemon — the same split `gate.rs` and `daemon/marker.rs` already keep over
/// `.trellis/acting-role`. The path is fixed in both.
fn dir(root: &Path) -> PathBuf {
    root.join(".trellis").join("runtime").join("waiting")
}

fn file(root: &Path, rel: &str) -> PathBuf {
    dir(root).join(slug(rel))
}

/// Filesystem-safe form of a task key or plan path
/// (`plan:plans/x.md` → `plan-plans-x.md`).
pub fn slug(key: &str) -> String {
    let s: String = key
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '.' | '_' => c,
            _ => '-',
        })
        .collect();
    s.trim_matches('-').to_string()
}

/// Declare a wait on `rel` lasting `for_secs` from `now`. Replaces any
/// standing lease: a session that says it again means the new deadline.
pub fn set(root: &Path, rel: &str, now: u64, for_secs: u64, what: &str) -> std::io::Result<Wait> {
    let wait = Wait {
        written_secs: now,
        expires_secs: now.saturating_add(for_secs),
        what: what.replace(['\n', '\r'], " ").trim().to_string(),
    };
    let dir = dir(root);
    std::fs::create_dir_all(&dir)?;
    let path = file(root, rel);
    // Written whole and renamed into place: a daemon reading mid-write would
    // otherwise see a lease with no deadline, which is the one way this could
    // fail *closed*.
    let tmp = path.with_extension("tmp");
    std::fs::write(
        &tmp,
        format!(
            "{} {} {}\n",
            wait.written_secs, wait.expires_secs, wait.what
        ),
    )?;
    std::fs::rename(&tmp, &path)?;
    Ok(wait)
}

/// Drop the lease. A lease that was never there is not an error — every
/// caller is making sure rather than undoing something it knows about.
pub fn clear(root: &Path, rel: &str) -> std::io::Result<()> {
    match std::fs::remove_file(file(root, rel)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

/// The standing lease, if there is a readable one. Absent, unreadable, and
/// malformed are all the same answer — no lease — because the fallback is the
/// plan's own status, which is never worse than what a half-read lease says.
pub fn get(root: &Path, rel: &str) -> Option<Wait> {
    let text = std::fs::read_to_string(file(root, rel)).ok()?;
    let line = text.lines().next()?;
    let mut fields = line.splitn(3, ' ');
    let written_secs: u64 = fields.next()?.trim().parse().ok()?;
    let expires_secs: u64 = fields.next()?.trim().parse().ok()?;
    let what = fields.next().unwrap_or("").trim().to_string();
    Some(Wait {
        written_secs,
        expires_secs,
        what,
    })
}

/// The lease still worth believing at `now` under `cap_secs`, if any.
pub fn live(root: &Path, rel: &str, now: u64, cap_secs: u64) -> Option<Wait> {
    get(root, rel).filter(|w| w.live(now, cap_secs))
}

/// Wall-clock unix seconds — `dates::today()`'s companion for the one clock
/// that is not a civil date. Deliberately not overridable by `TRELLIS_TODAY`:
/// this measures elapsed real time, and every caller passes it in so the
/// logic itself stays pure.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// `90s`, `5m`, `2h` — or a bare number of seconds. Small enough to own here
/// rather than take a dependency for, and strict enough that a typo is a
/// refusal rather than a surprising duration.
pub fn parse_duration(s: &str) -> anyhow::Result<u64> {
    let s = s.trim();
    let (digits, mult) = match s.chars().last() {
        Some('s') => (&s[..s.len() - 1], 1),
        Some('m') => (&s[..s.len() - 1], 60),
        Some('h') => (&s[..s.len() - 1], 3600),
        _ => (s, 1),
    };
    let n: u64 = digits
        .trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("not a duration: {s} (try 90s, 5m, 2h)"))?;
    if n == 0 {
        anyhow::bail!("a wait of zero is not a wait — clear it instead");
    }
    Ok(n * mult)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        tempfile::TempDir::new().unwrap()
    }

    #[test]
    fn a_lease_round_trips_and_clears() {
        let d = root();
        set(d.path(), "plans/ship-it.md", 1_000, 300, "cargo build").unwrap();
        let w = get(d.path(), "plans/ship-it.md").expect("written");
        assert_eq!(w.written_secs, 1_000);
        assert_eq!(w.expires_secs, 1_300);
        assert_eq!(w.what, "cargo build");
        clear(d.path(), "plans/ship-it.md").unwrap();
        assert_eq!(get(d.path(), "plans/ship-it.md"), None);
        // Clearing what is not there is how every caller uses it.
        clear(d.path(), "plans/ship-it.md").unwrap();
    }

    /// The cap is the operator's, and it binds from the moment the session
    /// asked — so tightening it does not leave yesterday's lease unbounded.
    #[test]
    fn the_cap_binds_a_lease_taken_before_it() {
        let w = Wait {
            written_secs: 1_000,
            expires_secs: 1_000 + 86_400,
            what: "a very long download".into(),
        };
        assert_eq!(w.deadline(7_200), 8_200);
        assert!(w.live(8_199, 7_200));
        assert!(!w.live(8_200, 7_200), "the cap wins over the ask");
        assert!(
            w.live(8_200, 172_800),
            "and the ask wins when it is shorter"
        );
        assert!(!w.live(1_000 + 86_400, 172_800));
    }

    /// Fails open: the fallback is the plan's own status, which is never
    /// worse than half a lease.
    #[test]
    fn an_unreadable_lease_is_no_lease() {
        let d = root();
        std::fs::create_dir_all(dir(d.path())).unwrap();
        std::fs::write(file(d.path(), "plans/x.md"), "not a lease\n").unwrap();
        assert_eq!(get(d.path(), "plans/x.md"), None);
        assert_eq!(get(d.path(), "plans/never-written.md"), None);
    }

    #[test]
    fn a_second_declaration_replaces_the_first() {
        let d = root();
        set(d.path(), "plans/x.md", 1_000, 60, "first").unwrap();
        set(d.path(), "plans/x.md", 1_050, 600, "second").unwrap();
        let w = get(d.path(), "plans/x.md").unwrap();
        assert_eq!((w.written_secs, w.expires_secs), (1_050, 1_650));
        assert_eq!(w.what, "second");
    }

    #[test]
    fn durations_are_seconds_minutes_or_hours() {
        assert_eq!(parse_duration("90").unwrap(), 90);
        assert_eq!(parse_duration("90s").unwrap(), 90);
        assert_eq!(parse_duration("5m").unwrap(), 300);
        assert_eq!(parse_duration(" 2h ").unwrap(), 7_200);
        assert!(parse_duration("soon").is_err());
        assert!(parse_duration("0m").is_err());
    }

    #[test]
    fn slugs_are_filesystem_safe() {
        assert_eq!(slug("plans/ship-it.md"), "plans-ship-it.md");
        assert_eq!(slug("plan:plans/ship-it.md"), "plan-plans-ship-it.md");
        assert_eq!(slug("ritual:conventions lint"), "ritual-conventions-lint");
    }
}
