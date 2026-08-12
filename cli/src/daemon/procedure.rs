//! The procedures the daemon renders into spawn prompts (decision 0050).
//!
//! `commands/act.md`, `commands/refine.md`, and `commands/ritual.md` are the
//! canonical contract for what a spawned session does — the same files the
//! interactive slash commands read. The binary embeds them at build time so a
//! spawned session needs no installed plugin: the default prompts carry the
//! computed facts first and `{procedure}` after, and `{procedure}` is one of
//! these renderings. `--plugin-root` (or `CLAUDE_PLUGIN_ROOT`) swaps a live
//! checkout in place of the embedded copies — refused when configured and
//! unreadable, the same posture as `{plugin_dir}`.
//!
//! Refine and ritual are resolver shells that end in "perform the
//! `/trellis:act` procedure", so their renderings carry the act body below
//! their own — the session reads one self-contained document.

use std::path::Path;

static ACT: &str = include_str!("../../../commands/act.md");
static REFINE: &str = include_str!("../../../commands/refine.md");
static RITUAL: &str = include_str!("../../../commands/ritual.md");

/// One rendered procedure per prompt kind, composed once at startup.
#[derive(Debug, Clone)]
pub struct Procedures {
    pub act: String,
    pub refine: String,
    pub ritual: String,
}

const JOINER: &str = "\n\n---\n\nThe /trellis:act procedure it delegates to (commands/act.md):\n\n";

impl Procedures {
    /// Embedded copies by default; the live checkout's `commands/` when a
    /// plugin root is known.
    pub fn load(plugin_root: Option<&Path>) -> anyhow::Result<Procedures> {
        let (act, refine, ritual) = match plugin_root {
            None => (ACT.to_string(), REFINE.to_string(), RITUAL.to_string()),
            Some(root) => {
                let read = |name: &str| -> anyhow::Result<String> {
                    let path = root.join("commands").join(name);
                    std::fs::read_to_string(&path).map_err(|e| {
                        anyhow::anyhow!(
                            "{}: {e} — the plugin root is configured, so a checkout without \
                             its commands is wrong, not a fallback",
                            path.display()
                        )
                    })
                };
                (read("act.md")?, read("refine.md")?, read("ritual.md")?)
            }
        };
        let act = strip_frontmatter(&act).trim().to_string();
        Ok(Procedures {
            refine: format!("{}{JOINER}{act}", strip_frontmatter(&refine).trim()),
            ritual: format!("{}{JOINER}{act}", strip_frontmatter(&ritual).trim()),
            act,
        })
    }
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
        let p = Procedures::load(None).unwrap();
        for body in [&p.act, &p.refine, &p.ritual] {
            assert!(!body.starts_with("---"), "frontmatter leaked");
            assert!(!body.contains("argument-hint:"), "frontmatter leaked");
        }
        // Sentinels the composition depends on: a restructure of the command
        // files that loses these must fail here, not in a live session.
        assert!(p.act.contains("Bind to the domain root"));
        assert!(p.act.contains("Adopt the holder"));
    }

    #[test]
    fn refine_and_ritual_carry_their_own_shell_and_the_act_body() {
        let p = Procedures::load(None).unwrap();
        assert!(p.refine.contains("The refinement contract"));
        assert!(p.refine.contains("Bind to the domain root"));
        assert!(p.ritual.contains("Find the row"));
        assert!(p.ritual.contains("Adopt the holder"));
    }

    #[test]
    fn a_configured_but_unreadable_checkout_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let e = Procedures::load(Some(dir.path())).unwrap_err();
        assert!(e.to_string().contains("commands"), "{e}");
    }

    #[test]
    fn a_live_checkout_wins_over_the_embedded_copy() {
        let dir = tempfile::tempdir().unwrap();
        let commands = dir.path().join("commands");
        std::fs::create_dir_all(&commands).unwrap();
        for name in ["act.md", "refine.md", "ritual.md"] {
            std::fs::write(commands.join(name), "---\ndescription: x\n---\nSENTINEL body\n")
                .unwrap();
        }
        let p = Procedures::load(Some(dir.path())).unwrap();
        assert!(p.act.contains("SENTINEL"));
        assert!(p.refine.contains("SENTINEL"));
    }
}
