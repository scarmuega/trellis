//! The runtime's half of `.trellis/acting-role` (decision 0045). The daemon
//! stamps the marker for every session it spawns, so a dispatched act never
//! spends a turn — or a file-creation permission dialog — writing it, and a
//! crashed session cannot leave it behind. The gate only tests existence
//! (`gate.rs`), so the format is informational: one line per live session,
//! `{role} {utc} {key}`, because `max_concurrent` allows sessions under
//! different roles at once. Lines in any other shape are an interactive
//! invocation's own marker (act.md's two-line form) and are never touched;
//! the file is deleted only when nothing of anyone's remains.

use std::path::Path;

fn path(root: &Path) -> std::path::PathBuf {
    root.join(".trellis").join("acting-role")
}

/// The marker has two writers since the split (dispatch and rituals are
/// separate processes — decision 0046), and every mutation is a
/// read-modify-write. A lockfile serializes them: create-exclusive, retried
/// briefly, stolen when stale — and everything fails open, because the
/// marker is attribution, not a lock on the work itself.
struct Lock(std::path::PathBuf);

impl Lock {
    fn take(root: &Path) -> Option<Lock> {
        let _ = std::fs::create_dir_all(root.join(".trellis"));
        let p = root.join(".trellis").join("acting-role.lock");
        for _ in 0..40 {
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&p)
            {
                Ok(_) => return Some(Lock(p.clone())),
                Err(_) => {
                    // A lock older than 10s belongs to a dead writer.
                    if let Ok(meta) = std::fs::metadata(&p) {
                        if let Ok(age) = meta.modified().and_then(|m| {
                            std::time::SystemTime::now()
                                .duration_since(m)
                                .map_err(|e| std::io::Error::other(e.to_string()))
                        }) {
                            if age.as_secs() > 10 {
                                let _ = std::fs::remove_file(&p);
                                continue;
                            }
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
        None
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// The key of a daemon line: everything after the role and timestamp,
/// because ritual keys carry spaces (`ritual:conventions lint`). Anything
/// not in that three-field shape — agent-written content included — has no
/// key and is never ours.
fn line_key(line: &str) -> Option<&str> {
    let mut fields = line.splitn(3, ' ');
    let (_role, _ts, key) = (fields.next()?, fields.next()?, fields.next()?);
    let key = key.trim();
    if key.is_empty() {
        None
    } else {
        Some(key)
    }
}

fn is_ours(line: &str, key: &str) -> bool {
    line_key(line) == Some(key)
}

/// Stamp a line for a freshly spawned session. Failures are logged by the
/// caller's judgment, never fatal: the marker is attribution, not a lock.
pub fn add(root: &Path, role: &str, timestamp: &str, key: &str) -> std::io::Result<()> {
    let _lock = Lock::take(root);
    let p = path(root);
    std::fs::create_dir_all(p.parent().expect(".trellis has a parent"))?;
    let mut content = std::fs::read_to_string(&p).unwrap_or_default();
    // Re-spawn after a write-off: one line per key, not one per attempt.
    content = content
        .lines()
        .filter(|l| !is_ours(l, key))
        .map(|l| format!("{l}\n"))
        .collect();
    content.push_str(&format!("{role} {timestamp} {key}\n"));
    std::fs::write(&p, content)
}

/// Drop the retired session's line; delete the file when nothing remains.
pub fn remove(root: &Path, key: &str) -> std::io::Result<()> {
    let _lock = Lock::take(root);
    let p = path(root);
    let Ok(content) = std::fs::read_to_string(&p) else {
        return Ok(());
    };
    let kept: String = content
        .lines()
        .filter(|l| !is_ours(l, key) && !l.trim().is_empty())
        .map(|l| format!("{l}\n"))
        .collect();
    if kept.is_empty() {
        std::fs::remove_file(&p)
    } else {
        std::fs::write(&p, kept)
    }
}

/// Startup reconciliation: a crash removes no lines, and adopted sessions
/// should keep theirs. Keep every daemon-format line *of this process's own
/// keyspace* (`plan:` for dispatch, `ritual:` for rituals — the other
/// process's live lines are not this one's to judge) whose key is still in
/// flight, drop the stale ones, and leave everything else alone.
pub fn reconcile(root: &Path, prefix: &str, live_keys: &[String]) -> std::io::Result<()> {
    let _lock = Lock::take(root);
    let p = path(root);
    let Ok(content) = std::fs::read_to_string(&p) else {
        return Ok(());
    };
    let kept: String = content
        .lines()
        .filter(|l| match line_key(l) {
            Some(key) if key.starts_with(prefix) => live_keys.iter().any(|k| k == key),
            _ => true,
        })
        .filter(|l| !l.trim().is_empty())
        .map(|l| format!("{l}\n"))
        .collect();
    if kept.is_empty() {
        std::fs::remove_file(&p)
    } else {
        std::fs::write(&p, kept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_come_and_go_per_session_and_the_file_with_them() {
        let dir = tempfile::TempDir::new().unwrap();
        add(dir.path(), "org/coder", "t1", "plan:plans/a.md").unwrap();
        add(dir.path(), "org/founder", "t2", "plan:plans/b.md").unwrap();
        let text = std::fs::read_to_string(path(dir.path())).unwrap();
        assert_eq!(text.lines().count(), 2);

        remove(dir.path(), "plan:plans/a.md").unwrap();
        let text = std::fs::read_to_string(path(dir.path())).unwrap();
        assert_eq!(text, "org/founder t2 plan:plans/b.md\n");

        remove(dir.path(), "plan:plans/b.md").unwrap();
        assert!(!path(dir.path()).exists(), "empty marker is no marker");
    }

    #[test]
    fn an_interactive_sessions_marker_is_never_ours_to_touch() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join(".trellis")).unwrap();
        std::fs::write(
            path(dir.path()),
            "org/founder\n2026-08-11T00:00:00Z\n",
        )
        .unwrap();
        add(dir.path(), "org/coder", "t", "plan:plans/a.md").unwrap();
        remove(dir.path(), "plan:plans/a.md").unwrap();
        let text = std::fs::read_to_string(path(dir.path())).unwrap();
        assert_eq!(text, "org/founder\n2026-08-11T00:00:00Z\n");
    }

    #[test]
    fn reconcile_drops_only_stale_lines_of_its_own_keyspace() {
        let dir = tempfile::TempDir::new().unwrap();
        add(dir.path(), "org/coder", "t", "plan:plans/dead.md").unwrap();
        add(dir.path(), "org/coder", "t", "plan:plans/live.md").unwrap();
        add(dir.path(), "org/steward", "t", "ritual:conventions lint").unwrap();
        reconcile(dir.path(), "plan:", &["plan:plans/live.md".into()]).unwrap();
        let text = std::fs::read_to_string(path(dir.path())).unwrap();
        assert!(!text.contains("dead"), "{text:?}");
        assert!(text.contains("plan:plans/live.md"));
        assert!(
            text.contains("ritual:conventions"),
            "the other process's line is not ours to judge: {text:?}"
        );
    }
}
