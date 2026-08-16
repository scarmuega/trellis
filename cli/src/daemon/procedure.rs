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
//!
//! One source, two renderings (decision 0058): spans fenced with
//! `<!-- interactive-only -->` … `<!-- /interactive-only -->` exist for the
//! interactive plane only and are stripped from every dispatch rendering —
//! the file the harness's command loader reads stays whole.

use std::path::Path;

static ACT: &str = include_str!("../../../commands/act.md");

const OPEN: &str = "<!-- interactive-only -->";
const CLOSE: &str = "<!-- /interactive-only -->";

/// The act body as dispatch renders it — embedded copy by default, the live
/// checkout's when a plugin root is known; frontmatter and interactive-only
/// spans stripped.
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
    let body = strip_interactive_only(strip_frontmatter(&text))?;
    Ok(body.trim().to_string())
}

/// Remove `<!-- interactive-only -->` spans, marker lines included. Each
/// marker must stand alone on its line, opens must close, and spans must not
/// nest — anything else is a startup error, never a silent half-strip: a
/// prompt carrying half a fenced sentence is worse than a daemon that
/// refuses to start.
fn strip_interactive_only(text: &str) -> anyhow::Result<String> {
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    for line in text.lines() {
        let trimmed = line.trim();
        let (opens, closes) = (trimmed == OPEN, trimmed == CLOSE);
        if !opens && !closes && (line.contains(OPEN) || line.contains(CLOSE)) {
            anyhow::bail!(
                "act.md: an interactive-only marker must stand alone on its line: {line:?}"
            );
        }
        match (opens, closes, skipping) {
            (true, _, true) => anyhow::bail!("act.md: nested {OPEN} — spans must not nest"),
            (true, _, false) => skipping = true,
            (_, true, false) => anyhow::bail!("act.md: {CLOSE} without an open marker"),
            (_, true, true) => skipping = false,
            _ if skipping => {}
            _ => {
                out.push_str(line);
                out.push('\n');
            }
        }
    }
    if skipping {
        anyhow::bail!("act.md: unclosed {OPEN} — the span never ends");
    }
    // Collapse the blank runs a stripped span can leave between paragraphs.
    while out.contains("\n\n\n") {
        out = out.replace("\n\n\n", "\n\n");
    }
    Ok(out)
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
    fn interactive_only_spans_never_reach_a_dispatch_rendering() {
        let act = load(None).unwrap();
        // The fenced spans and their markers are gone…
        assert!(!act.contains("interactive-only"), "marker leaked");
        assert!(!act.contains("$ARGUMENTS"), "argument parsing leaked");
        assert!(!act.contains("No input means"), "no-input case leaked");
        assert!(
            !act.contains("list the roles under"),
            "ghost-role branch leaked"
        );
        assert!(
            !act.contains("Remove it in step 7"),
            "marker mechanics leaked"
        );
        // …and the branches every dispatch flavor still needs are not.
        assert!(
            act.contains("never impersonate"),
            "human-holder branch lost"
        );
        assert!(
            act.contains("delegated execution (decision 0057)"),
            "errand delegation branch lost"
        );
        assert!(
            act.contains("leave the file alone"),
            "dispatch marker rule lost"
        );
    }

    #[test]
    fn an_unclosed_marker_is_refused() {
        let e = strip_interactive_only("a\n<!-- interactive-only -->\nb\n").unwrap_err();
        assert!(e.to_string().contains("unclosed"), "{e}");
    }

    #[test]
    fn a_stray_close_and_a_mid_line_marker_are_refused() {
        let e = strip_interactive_only("a\n<!-- /interactive-only -->\n").unwrap_err();
        assert!(e.to_string().contains("without an open"), "{e}");
        let e = strip_interactive_only("text <!-- interactive-only -->\n").unwrap_err();
        assert!(e.to_string().contains("stand alone"), "{e}");
    }

    #[test]
    fn a_stripped_span_leaves_no_blank_scar() {
        let got = strip_interactive_only(
            "one\n<!-- interactive-only -->\ntwo\n<!-- /interactive-only -->\nthree\n",
        )
        .unwrap();
        assert_eq!(got, "one\nthree\n");
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
