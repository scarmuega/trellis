//! The wall-clock command: `rituals.md` cadences decide what runs, last-fired
//! dates decide when it runs again, and a cadence with no day count never
//! runs at all. `trellis rituals` is one idempotent pass (decision 0046) —
//! time is `TRELLIS_TODAY`, so none of this waits for a real week.

mod common;

use common::{Fixture, ANCHOR};

const RITUALS: &str = "---
provenance: authored
owner: org/founder
---
# Rituals

| ritual          | cadence   | executor    | procedure                  |
|-----------------|-----------|-------------|----------------------------|
| conventions lint| weekly    | org/steward | run the lint checklist     |
| plan dispatch   | daily     | org/steward | scan ready plans           |
| incident review | on demand | org/founder | whatever the incident asks |
";

fn fixture() -> Fixture {
    let f = Fixture::healthy();
    f.write("rituals.md", RITUALS);
    let harness = f.fake_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\", \"{{model}}\", \"{{effort}}\", \"{{budget}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\", \"{{prompt}}\"]\n"
        ),
    );
    f
}

fn rituals_run(f: &Fixture) -> Vec<String> {
    f.invocations()
        .into_iter()
        .filter_map(|argv| argv.first().cloned())
        .collect()
}

#[test]
fn a_first_pass_fires_every_scheduled_row() {
    let f = fixture();
    f.rituals_once(ANCHOR, &[]);

    assert_eq!(rituals_run(&f), vec!["conventions lint"]);
    let argv = &f.invocations()[0];
    // The prompt element line-splits in the harness's argv log; its first
    // line is the discriminating header the default renders (decision 0050),
    // and the procedure body follows in the same element.
    assert_eq!(
        argv[1],
        "ritual conventions lint — executed by org/steward (trellis runtime)."
    );
    let prompt = argv[1..].join("\n");
    assert!(prompt.contains("read its row"), "{prompt}");
    assert!(prompt.contains("Bind to the domain root"), "{prompt}");

    let state = f.rituals_state();
    assert_eq!(
        state["runs"]["ritual:conventions lint"]["last_fired"],
        ANCHOR
    );
    assert_eq!(state["runs"]["ritual:conventions lint"]["last_exit"], 0);
    assert!(
        state["runs"].get("dispatch").is_none(),
        "dispatch has no calendar key — it is the dispatcher's loop (0046)"
    );
}

#[test]
fn a_row_already_fired_today_does_not_fire_again() {
    let f = fixture();
    f.rituals_once(ANCHOR, &[]);
    let first = f.invocations().len();
    f.rituals_once(ANCHOR, &[]);
    assert_eq!(
        f.invocations().len(),
        first,
        "a second pass changes nothing"
    );
}

#[test]
fn a_weekly_row_returns_on_its_cadence_and_not_before() {
    let f = fixture();
    f.rituals_once(ANCHOR, &[]);
    f.rituals_once("2026-08-09", &[]); // six days on: not yet
    assert_eq!(rituals_run(&f), vec!["conventions lint"]);

    f.rituals_once("2026-08-10", &[]); // seven days on
    assert_eq!(
        rituals_run(&f),
        vec!["conventions lint", "conventions lint"]
    );
}

#[test]
fn a_window_missed_while_the_daemon_was_down_fires_once_not_once_per_window() {
    let f = fixture();
    f.rituals_once(ANCHOR, &[]);
    // Two months later, after eight missed weekly windows.
    f.rituals_once("2026-10-05", &[]);
    assert_eq!(
        rituals_run(&f),
        vec!["conventions lint", "conventions lint"],
        "one catch-up run, not eight"
    );
}

#[test]
fn catchup_skip_records_the_missed_window_without_running_it() {
    let f = fixture();
    let harness = ".trellis/bin/harness";
    f.write(
        "runtime.toml",
        &format!(
            "[scheduler]\ncatchup = \"skip\"\n\n\
             [harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\"]\n"
        ),
    );
    f.rituals_once(ANCHOR, &[]);
    let out = f.rituals_once("2026-10-05", &[]);

    assert_eq!(rituals_run(&f), vec!["conventions lint"], "nothing re-ran");
    assert!(out.contains("window missed"), "{out}");
    assert_eq!(
        f.rituals_state()["runs"]["ritual:conventions lint"]["last_fired"],
        "2026-10-05",
        "recorded, so the next fire is a full period out rather than immediate"
    );
}

#[test]
fn a_cadence_with_no_day_count_never_fires_and_is_reported() {
    let f = fixture();
    f.rituals_once(ANCHOR, &[]);
    assert!(
        !rituals_run(&f).contains(&"incident review".to_string()),
        "'on demand' is not a schedule"
    );
    let state = f.rituals_state();
    assert!(state["runs"].get("ritual:incident review").is_none());
}

#[test]
fn a_dry_run_reports_the_command_and_changes_nothing() {
    let f = fixture();
    let out = f.rituals_once(ANCHOR, &["--dry-run"]);

    assert!(out.contains("would run ritual conventions lint"), "{out}");
    assert!(f.invocations().is_empty(), "nothing was spawned");
    assert_eq!(
        f.rituals_state(),
        serde_json::Value::Null,
        "nothing was remembered"
    );
}

#[test]
fn the_default_config_needs_no_plugin_at_all() {
    let f = Fixture::healthy();
    f.write("rituals.md", RITUALS);
    // No runtime.toml, no CLAUDE_PLUGIN_ROOT, no installed plugin: the
    // default prompt is self-contained — the procedure rendered from the
    // binary's embedded commands (decision 0050), no slash command to
    // resolve, no checkout named.
    let out = f.rituals_once(ANCHOR, &["--dry-run"]);

    assert!(
        out.contains("ritual conventions lint — executed by org/steward"),
        "{out}"
    );
    assert!(out.contains("Bind to the domain root"), "{out}");
    // The procedure prose may *mention* /trellis:act; the prompt must not
    // *open* with a slash command the harness would have to resolve.
    assert!(!out.contains("-p '/trellis:"), "{out}");
    assert!(out.contains("--permission-mode acceptEdits"), "{out}");
    assert!(
        !out.contains("--plugin-dir"),
        "the default points at no checkout: {out}"
    );
}

#[test]
fn a_template_naming_the_plugin_without_one_configured_refuses_to_start() {
    let f = Fixture::healthy();
    f.write("rituals.md", RITUALS);
    // Naming {plugin_dir} is how a domain operating against an uninstalled
    // checkout opts in; doing so without supplying one must refuse rather
    // than quietly run against whatever plugin the harness finds.
    f.write(
        "runtime.toml",
        r#"[harness]
ritual_cmd = ["claude", "--plugin-dir", "{plugin_dir}"]
"#,
    );
    let out = f
        .bin()
        .args(["rituals"])
        .env_remove("CLAUDE_PLUGIN_ROOT")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no plugin checkout is known"), "{err}");
}

#[test]
fn an_instance_keeping_the_slash_spelling_renders_exactly_as_before() {
    let f = Fixture::healthy();
    f.write("rituals.md", RITUALS);
    // The migration story (decision 0050): the daemon always supplies every
    // placeholder; a prompt override that still names the plugin spelling
    // renders untouched, procedure and all unused.
    f.write(
        "runtime.toml",
        "[prompts]\nritual = \"/trellis:ritual {ritual}\"\n",
    );
    let out = f.rituals_once(ANCHOR, &["--dry-run"]);
    assert!(
        out.contains("claude -p '/trellis:ritual conventions lint'"),
        "{out}"
    );
}

#[test]
fn a_domain_may_still_point_its_sessions_at_a_checkout() {
    let f = Fixture::healthy();
    f.write("rituals.md", RITUALS);
    f.write(
        "runtime.toml",
        r#"[harness]
ritual_cmd = ["claude", "-p", "{prompt}", "--plugin-dir", "{plugin_dir}"]
"#,
    );
    // A configured checkout must carry its commands (decision 0050): the
    // {procedure} source swaps to it, so a bare directory is refused.
    let checkout = tempfile::tempdir().unwrap();
    let commands = checkout.path().join("commands");
    std::fs::create_dir_all(&commands).unwrap();
    for name in ["act.md", "refine.md", "ritual.md"] {
        std::fs::write(
            commands.join(name),
            "---\ndescription: probe\n---\nCHECKOUT-SENTINEL procedure body\n",
        )
        .unwrap();
    }
    let root = checkout.path().display().to_string();
    let out = f.rituals_once(ANCHOR, &["--dry-run", "--plugin-root", &root]);
    assert!(out.contains(&format!("--plugin-dir {root}")), "{out}");
    assert!(
        out.contains("CHECKOUT-SENTINEL"),
        "the live checkout's procedure rides the prompt: {out}"
    );

    // The same config against a checkout with no commands refuses to start.
    let empty = tempfile::tempdir().unwrap();
    let out = f.rituals_expect_failure(ANCHOR, &[
        "--dry-run",
        "--plugin-root",
        &empty.path().display().to_string(),
    ]);
    assert!(out.contains("commands"), "{out}");
}
