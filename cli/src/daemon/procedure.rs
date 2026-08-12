//! The one procedure the daemon renders into spawn prompts: `act`.
//!
//! `commands/act.md` is the runtime's only primitive (decision 0051) — every
//! spawn prompt's `{procedure}` is its body, and everything an errand adds on
//! top (the refine contract, the ritual framing, whatever an instance
//! invents) lives in that errand's prompt template in `runtime.toml`, not in
//! the binary. The file is embedded at build time so a spawned session needs
//! no installed plugin (decision 0050); `--plugin-root` (or
//! `CLAUDE_PLUGIN_ROOT`) swaps a live checkout's copy — refused when
//! configured and unreadable, the same posture as `{plugin_dir}`.

use std::path::Path;

static ACT: &str = include_str!("../../../commands/act.md");

/// The act body — embedded copy by default, the live checkout's when a
/// plugin root is known.
pub fn load(plugin_root: Option<&Path>) -> anyhow::Result<String> {
    let text = match plugin_root {
        None => ACT.to_string(),
        Some(root) => {
            let path = root.join("commands").join("act.md");
            std::fs::read_to_string(&path).map_err(|e| {
                anyhow::anyhow!(
                    "{}: {e} — the plugin root is configured, so a checkout without \
                     commands/act.md is wrong, not a fallback",
                    path.display()
                )
            })?
        }
    };
    Ok(strip_frontmatter(&text).trim().to_string())
}

/// Drop the leading `---` block: the frontmatter addresses the harness's
/// command loader, not the session.
fn strip_frontmatter(text: &str) -> &str {
    let Some(rest) = text.strip_prefix("---\n") else {
        return text;
    };
    match rest.find("\n---\n") {
        Some(end) => &rest[end + "\n---\n".len()..],
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_is_stripped_and_the_body_survives() {
        let act = load(None).unwrap();
        assert!(!act.starts_with("---"), "frontmatter leaked");
        assert!(!act.contains("argument-hint:"), "frontmatter leaked");
        // Sentinels the prompts depend on: a restructure of act.md that
        // loses these must fail here, not in a live session.
        assert!(act.contains("Bind to the domain root"));
        assert!(act.contains("Adopt the holder"));
    }

    #[test]
    fn a_configured_but_unreadable_checkout_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let e = load(Some(dir.path())).unwrap_err();
        assert!(e.to_string().contains("act.md"), "{e}");
    }

    #[test]
    fn a_live_checkout_wins_over_the_embedded_copy() {
        let dir = tempfile::tempdir().unwrap();
        let commands = dir.path().join("commands");
        std::fs::create_dir_all(&commands).unwrap();
        std::fs::write(
            commands.join("act.md"),
            "---\ndescription: x\n---\nSENTINEL body\n",
        )
        .unwrap();
        assert_eq!(load(Some(dir.path())).unwrap(), "SENTINEL body");
    }
}
