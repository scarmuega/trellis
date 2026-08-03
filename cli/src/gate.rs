//! `trellis gate` — the PreToolUse hook, an exact port of `hooks/gate.mjs`.
//!
//! Guards only what needs no judgment and no role attribution:
//!   1. a *committed* accepted decision is never edited (spec rule 6);
//!      an uncommitted draft stays editable
//!   2. `provenance: generated` artifacts are written only under an
//!      acting-role marker (`.trellis/acting-role`)
//!   3. a new `.md` artifact without frontmatter gets a non-blocking warning
//!
//! Outside a Trellis root the gate is inert. Fails open: every error path,
//! panic included, allows — a broken gate must never brick a session.

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::root::Root;

/// Byte cap on frontmatter reads — the gate never loads whole files.
const FM_CAP: usize = 4096;

fn frontmatter_head(text: &str) -> Option<&str> {
    let head = &text[..text.len().min(FM_CAP)];
    let re = regex::Regex::new(r"(?s)\A---\r?\n(.*?)\r?\n---").ok()?;
    Some(re.captures(head)?.get(1)?.as_str())
}

fn fm_matches(text: &str, field_re: &str) -> bool {
    frontmatter_head(text)
        .map(|fm| {
            regex::Regex::new(field_re)
                .map(|re| re.is_match(fm))
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

fn read_head(path: &Path) -> String {
    use std::io::Read;
    let Ok(mut f) = std::fs::File::open(path) else {
        return String::new();
    };
    let mut buf = vec![0u8; FM_CAP];
    let n = f.read(&mut buf).unwrap_or(0);
    buf.truncate(n);
    String::from_utf8_lossy(&buf).into_owned()
}

/// Append-only freezes a decision once its accepted state is *committed*.
/// HEAD, not the worktree or index: a decision drafted/accepted this session
/// is not yet in HEAD and stays editable. Outside a git repo the "committed"
/// notion is meaningless — fall back to always-frozen rather than silently
/// dropping the guarantee.
fn committed_decision_is_accepted(root: &Path, rel: &str) -> bool {
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(args)
            .output()
            .ok()?;
        if out.status.success() {
            Some(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            None
        }
    };
    if git(&["rev-parse", "--git-dir"]).is_none() {
        return true; // not a git repo (or git missing): frozen
    }
    match git(&["show", &format!("HEAD:{rel}")]) {
        None => false, // not in HEAD → uncommitted → editable
        Some(text) => fm_matches(&text, r"(?m)^status:\s*accepted\b"),
    }
}

fn deny(reason: &str) -> Option<String> {
    Some(
        serde_json::json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "deny",
                "permissionDecisionReason": reason,
            }
        })
        .to_string(),
    )
}

fn warn(message: &str) -> Option<String> {
    Some(serde_json::json!({ "systemMessage": message }).to_string())
}

/// Process one PreToolUse payload; the returned string (if any) goes to
/// stdout. Never errors — any anomaly allows.
pub fn process(input: &str) -> Option<String> {
    let payload: serde_json::Value = serde_json::from_str(input).ok()?;
    let tool_input = payload.get("tool_input")?;
    let file_path = tool_input.get("file_path")?.as_str()?;
    if file_path.is_empty() {
        return None;
    }

    let cwd = payload
        .get("cwd")
        .and_then(|c| c.as_str())
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())?;
    let abs = cwd.join(file_path);

    let root = Root::discover_from(abs.parent()?)?;
    // The plugin's own template/ is shaped like a root; don't gate plugin dev.
    if root.is_plugin_template() {
        return None;
    }

    let rel = root.rel(&abs)?;
    let exists = abs.exists();

    // 1. decisions/ is append-only: a committed accepted decision is never
    //    edited. An uncommitted draft (not yet in HEAD) is still part of the
    //    session.
    let decision_re = regex::Regex::new(r"^decisions/\d{4}-[^/]+\.md$").ok()?;
    if decision_re.is_match(&rel) && exists {
        let head = read_head(&abs);
        if fm_matches(&head, r"(?m)^status:\s*accepted\b")
            && committed_decision_is_accepted(&root.path, &rel)
        {
            return deny(&format!(
                "{rel} is a committed accepted decision — decisions/ is append-only (spec rule 6). \
                 Supersede it with a new numbered decision that names this one instead of editing it."
            ));
        }
    }

    // 2. Generated artifacts: only an attributed acting role may write them.
    if exists && fm_matches(&read_head(&abs), r"(?m)^provenance:\s*generated\b") {
        let marker = root.path.join(".trellis").join("acting-role");
        if !marker.exists() {
            return deny(&format!(
                "{rel} carries provenance: generated — hand-edits are blocked. \
                 Fix the generator or the source artifact instead; a mandated agent \
                 regenerating it must act through /trellis:act <role> so the write is attributed."
            ));
        }
    }

    // 3. New markdown artifact without frontmatter: warn, don't block.
    if !exists
        && payload.get("tool_name").and_then(|t| t.as_str()) == Some("Write")
        && rel.ends_with(".md")
        && !rel.starts_with('.')
    {
        if let Some(content) = tool_input.get("content").and_then(|c| c.as_str()) {
            let has_fm = regex::Regex::new(r"(?s)\A---\r?\n.*?\bprovenance:")
                .map(|re| re.is_match(content))
                .unwrap_or(true);
            if !has_fm {
                return warn(&format!(
                    "trellis gate: new artifact {rel} declares no provenance/owner frontmatter — \
                     every artifact in a Trellis root carries provenance: and owner: (see the instance's conventions.md)."
                ));
            }
        }
    }

    None
}

/// Hook entrypoint: read stdin, respond on stdout, always exit 0.
pub fn run() -> ! {
    let result = std::panic::catch_unwind(|| {
        use std::io::Read;
        let mut input = String::new();
        // Size-capped read: a runaway stdin must not stall the hook.
        let _ = std::io::stdin()
            .take(4 * 1024 * 1024)
            .read_to_string(&mut input);
        if std::env::var("TRELLIS_GATE_TEST_PANIC").is_ok() {
            panic!("test-injected gate panic");
        }
        process(&input)
    });
    if let Ok(Some(out)) = result {
        print!("{out}");
    }
    std::process::exit(0);
}
