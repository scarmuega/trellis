//! Placeholder substitution for harness command templates.
//!
//! A template is an argv array, never a shell string: a placeholder fills
//! exactly one argument slot, so nothing an artifact carries can become a
//! shell word. `{name}` is substituted only for the known placeholders below;
//! any other `{…}` run is a configuration error caught at startup rather than
//! at spawn time. There is no escape form — a literal `{word}` in a prompt is
//! not expressible, which is a price paid for having no quoting rules at all.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Every substitution a template may name.
pub const PLACEHOLDERS: &[&str] = &[
    "prompt",
    "model",
    "effort",
    "budget",
    "plugin_dir",
    "root",
    "plan",
    "owner",
    "ritual",
    "complexity",
    // The session's own back-channel, as a `--mcp-config` value (decision
    // 0041). Naming it is what opts a binding into the channel; a template
    // that omits it runs exactly as it did before.
    "mcp",
    // Where the acting role's escalations go, read from its mandate at scan
    // time (decision 0045) — a fact the session would otherwise spend a read
    // deriving. Falls back to the role itself when the mandate is silent, so
    // it always renders.
    "escalate_to",
    // The plan's effective automation class, computed by the scan — the
    // derivation whose instructions used to cost the session the whole
    // conventions read. Empty for rituals, which carry no plan.
    "automation",
    // The operator's refinement instruction, relayed verbatim into the
    // prompt (decision 0048). Set only for refine sessions.
    "instruction",
];

static PATTERN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"\{([a-z_]+)\}").unwrap());

/// The values one invocation substitutes.
#[derive(Debug, Default, Clone)]
pub struct Vars(HashMap<&'static str, String>);

impl Vars {
    pub fn new() -> Vars {
        Vars(HashMap::new())
    }

    pub fn set(mut self, key: &'static str, value: impl Into<String>) -> Vars {
        debug_assert!(PLACEHOLDERS.contains(&key), "{key} is not a placeholder");
        self.0.insert(key, value.into());
        self
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).map(String::as_str)
    }
}

/// Reject a template naming a placeholder that does not exist. Startup-time
/// check: a misspelled `{modell}` must not survive until a plan is dispatched.
pub fn check(what: &str, template: &[String]) -> anyhow::Result<()> {
    for element in template {
        for cap in PATTERN.captures_iter(element) {
            let name = &cap[1];
            if !PLACEHOLDERS.contains(&name) {
                anyhow::bail!(
                    "{what} names {{{name}}}, which is not a placeholder ({})",
                    PLACEHOLDERS.join(", ")
                );
            }
        }
    }
    Ok(())
}

/// Substitute into every element. A placeholder the caller left unset is an
/// error, not an empty string: `{plan}` in a ritual template is a mistake
/// worth reporting, never a silently truncated command line.
pub fn substitute(what: &str, template: &[String], vars: &Vars) -> anyhow::Result<Vec<String>> {
    check(what, template)?;
    let mut out = Vec::with_capacity(template.len());
    for element in template {
        let mut missing = None;
        let rendered =
            PATTERN.replace_all(element, |cap: &regex::Captures| match vars.get(&cap[1]) {
                Some(v) => v.to_string(),
                None => {
                    missing.get_or_insert_with(|| cap[1].to_string());
                    String::new()
                }
            });
        if let Some(name) = missing {
            anyhow::bail!("{what} names {{{name}}}, which this invocation does not set");
        }
        out.push(rendered.into_owned());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn substitutes_one_slot_per_argument() {
        let vars = Vars::new()
            .set("prompt", "/trellis:act coder advance plans/x.md")
            .set("model", "opus");
        let got = substitute(
            "act_cmd",
            &t(&["claude", "-p", "{prompt}", "{model}"]),
            &vars,
        )
        .unwrap();
        assert_eq!(
            got,
            vec![
                "claude",
                "-p",
                "/trellis:act coder advance plans/x.md",
                "opus"
            ]
        );
    }

    #[test]
    fn a_prompt_carrying_spaces_stays_one_argument() {
        let vars = Vars::new().set("plan", "plans/a b.md");
        let got = substitute("act", &t(&["advance {plan} now"]), &vars).unwrap();
        assert_eq!(got, vec!["advance plans/a b.md now"]);
    }

    #[test]
    fn unknown_placeholder_is_a_startup_error() {
        let e = check("act_cmd", &t(&["--model", "{modell}"])).unwrap_err();
        assert!(e.to_string().contains("not a placeholder"), "{e}");
    }

    #[test]
    fn unset_placeholder_refuses_rather_than_blanking() {
        let e = substitute("ritual_cmd", &t(&["{plan}"]), &Vars::new()).unwrap_err();
        assert!(e.to_string().contains("does not set"), "{e}");
    }

    #[test]
    fn text_that_is_not_a_placeholder_passes_through() {
        let got = substitute("prompt", &t(&["a {Braced} literal"]), &Vars::new()).unwrap();
        assert_eq!(got, vec!["a {Braced} literal"]);
    }
}
