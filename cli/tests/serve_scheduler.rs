//! The daemon's clock: `rituals.md` cadences decide what runs, last-fired
//! dates decide when it runs again, and a cadence with no day count never
//! runs at all. Time is `TRELLIS_TODAY` and passes are `--once`, so none of
//! this waits for a real week.

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
    f.serve_once(ANCHOR, &[]);

    assert_eq!(rituals_run(&f), vec!["conventions lint"]);
    let argv = &f.invocations()[0];
    assert_eq!(
        argv[1], "/trellis:ritual conventions lint",
        "the prompt is rendered and stays one argument"
    );

    let state = f.state();
    assert_eq!(
        state["runs"]["ritual:conventions lint"]["last_fired"],
        ANCHOR
    );
    assert_eq!(state["runs"]["ritual:conventions lint"]["last_exit"], 0);
    assert_eq!(
        state["runs"]["dispatch"]["last_fired"], ANCHOR,
        "the scan ran even though this fixture has no ready plans"
    );
}

#[test]
fn a_row_already_fired_today_does_not_fire_again() {
    let f = fixture();
    f.serve_once(ANCHOR, &[]);
    let first = f.invocations().len();
    f.serve_once(ANCHOR, &[]);
    assert_eq!(
        f.invocations().len(),
        first,
        "a second pass changes nothing"
    );
}

#[test]
fn a_weekly_row_returns_on_its_cadence_and_not_before() {
    let f = fixture();
    f.serve_once(ANCHOR, &[]);
    f.serve_once("2026-08-09", &[]); // six days on: not yet
    assert_eq!(rituals_run(&f), vec!["conventions lint"]);

    f.serve_once("2026-08-10", &[]); // seven days on
    assert_eq!(
        rituals_run(&f),
        vec!["conventions lint", "conventions lint"]
    );
}

#[test]
fn a_window_missed_while_the_daemon_was_down_fires_once_not_once_per_window() {
    let f = fixture();
    f.serve_once(ANCHOR, &[]);
    // Two months later, after eight missed weekly windows.
    f.serve_once("2026-10-05", &[]);
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
    f.serve_once(ANCHOR, &[]);
    let out = f.serve_once("2026-10-05", &[]);

    assert_eq!(rituals_run(&f), vec!["conventions lint"], "nothing re-ran");
    assert!(out.contains("window missed"), "{out}");
    assert_eq!(
        f.state()["runs"]["ritual:conventions lint"]["last_fired"],
        "2026-10-05",
        "recorded, so the next fire is a full period out rather than immediate"
    );
}

#[test]
fn a_cadence_with_no_day_count_never_fires_and_is_reported() {
    let f = fixture();
    f.serve_once(ANCHOR, &[]);
    assert!(
        !rituals_run(&f).contains(&"incident review".to_string()),
        "'on demand' is not a schedule"
    );
    let state = f.state();
    assert!(state["runs"].get("ritual:incident review").is_none());
}

#[test]
fn a_dry_run_reports_the_command_and_changes_nothing() {
    let f = fixture();
    let out = f.serve_once(ANCHOR, &["--dry-run"]);

    assert!(out.contains("would run ritual conventions lint"), "{out}");
    assert!(f.invocations().is_empty(), "nothing was spawned");
    assert_eq!(f.state(), serde_json::Value::Null, "nothing was remembered");
}

#[test]
fn the_default_config_runs_the_installed_plugin_without_pointing_at_a_checkout() {
    let f = Fixture::healthy();
    f.write("rituals.md", RITUALS);
    // No runtime.toml at all, and no CLAUDE_PLUGIN_ROOT: the defaults are the
    // reference binding's, and they must start on a machine where the plugin
    // is simply installed in the harness — which is every machine a local
    // daemon runs on.
    let out = f.serve_once(ANCHOR, &["--dry-run"]);

    assert!(
        out.contains("claude -p '/trellis:ritual conventions lint'"),
        "{out}"
    );
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
        .args(["serve", "--once", "--no-http"])
        .env_remove("CLAUDE_PLUGIN_ROOT")
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("no plugin checkout is known"), "{err}");
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
    let out = f.serve_once(ANCHOR, &["--dry-run", "--plugin-root", "/tmp/plugin"]);
    assert!(out.contains("--plugin-dir /tmp/plugin"), "{out}");
}
