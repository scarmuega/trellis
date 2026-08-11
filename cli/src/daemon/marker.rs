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

/// Is this line ours for `key`? Daemon lines carry the key as their third
/// field; anything else — agent-written content included — is not ours.
fn is_ours(line: &str, key: &str) -> bool {
    let fields: Vec<&str> = line.split_whitespace().collect();
    fields.len() == 3 && fields[2] == key
}

/// Stamp a line for a freshly spawned session. Failures are logged by the
/// caller's judgment, never fatal: the marker is attribution, not a lock.
pub fn add(root: &Path, role: &str, timestamp: &str, key: &str) -> std::io::Result<()> {
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
/// should keep theirs. Keep every daemon-format line whose key is still in
/// flight, drop the stale ones, and leave non-daemon lines alone.
pub fn reconcile(root: &Path, live_keys: &[String]) -> std::io::Result<()> {
    let p = path(root);
    let Ok(content) = std::fs::read_to_string(&p) else {
        return Ok(());
    };
    let kept: String = content
        .lines()
        .filter(|l| {
            let fields: Vec<&str> = l.split_whitespace().collect();
            let daemon_line = fields.len() == 3 && fields[2].contains(':');
            !daemon_line || live_keys.iter().any(|k| k == fields[2])
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
    fn reconcile_drops_only_stale_daemon_lines() {
        let dir = tempfile::TempDir::new().unwrap();
        add(dir.path(), "org/coder", "t", "plan:plans/dead.md").unwrap();
        add(dir.path(), "org/coder", "t", "plan:plans/live.md").unwrap();
        reconcile(dir.path(), &["plan:plans/live.md".into()]).unwrap();
        let text = std::fs::read_to_string(path(dir.path())).unwrap();
        assert_eq!(text, "org/coder t plan:plans/live.md\n");
    }
}
