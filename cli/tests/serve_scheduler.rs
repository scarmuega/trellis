//! The wall-clock command: `rituals.md` cadences decide what runs, last-fired
//! dates decide when it runs again, and a cadence with no day count never
//! runs at all. `trellis rituals` is one idempotent pass (decision 0046) —
//! time is `TRELLIS_TODAY`, so none of this waits for a real week.

mod common;

use common::{FakeHerdr, Fixture, ANCHOR};

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

fn fixture() -> (Fixture, FakeHerdr) {
    let f = Fixture::healthy();
    f.write("rituals.md", RITUALS);
    // The budget is no longer a flag anywhere (decision 0043's trade, now
    // the only story): an interactive session has nothing to enforce it.
    let herdr = common::wire_herdr_split(
        &f,
        "\"{plan}\"",
        "\"{ritual}\"",
        "",
        &["working", "idle"],
        None,
    );
    (f, herdr)
}

/// Which rituals fired, read off the flags each session was started with.
fn rituals_run(h: &FakeHerdr) -> Vec<String> {
    h.started_args()
        .into_iter()
        .filter_map(|args| args.first().cloned())
        .collect()
}

#[test]
fn a_first_pass_fires_every_scheduled_row() {
    let (f, herdr) = fixture();
    f.rituals_once(ANCHOR, &[]);

    assert_eq!(rituals_run(&herdr), vec!["conventions lint"]);
    let prompt = &herdr.prompts()[0];
    assert_eq!(
        prompt.lines().next().unwrap(),
        "ritual conventions lint — executed by org/steward (trellis runtime)."
    );
    assert!(prompt.contains("read its row"), "{prompt}");
    assert!(prompt.contains("Bind to the domain root"), "{prompt}");

    let state = f.rituals_state();
    assert_eq!(
        state["runs"]["ritual:conventions lint"]["last_fired"],
        ANCHOR
    );
    assert_eq!(
        state["runs"]["ritual:conventions lint"]["last_outcome"], "settled",
        "a ritual leaves no artifact to read a verdict from: settling is its end"
    );
    assert!(
        state["runs"].get("dispatch").is_none(),
        "dispatch has no calendar key — it is the dispatcher's loop (0046)"
    );
}

#[test]
fn a_row_already_fired_today_does_not_fire_again() {
    let (f, herdr) = fixture();
    f.rituals_once(ANCHOR, &[]);
    let first = herdr.started_args().len();
    f.rituals_once(ANCHOR, &[]);
    assert_eq!(
        herdr.started_args().len(),
        first,
        "a second pass changes nothing"
    );
}

#[test]
fn a_weekly_row_returns_on_its_cadence_and_not_before() {
    let (f, herdr) = fixture();
    f.rituals_once(ANCHOR, &[]);
    f.rituals_once("2026-08-09", &[]); // six days on: not yet
    assert_eq!(rituals_run(&herdr), vec!["conventions lint"]);

    f.rituals_once("2026-08-10", &[]); // seven days on
    assert_eq!(
        rituals_run(&herdr),
        vec!["conventions lint", "conventions lint"]
    );
}

#[test]
fn a_window_missed_while_the_daemon_was_down_fires_once_not_once_per_window() {
    let (f, herdr) = fixture();
    f.rituals_once(ANCHOR, &[]);
    // Two months later, after eight missed weekly windows.
    f.rituals_once("2026-10-05", &[]);
    assert_eq!(
        rituals_run(&herdr),
        vec!["conventions lint", "conventions lint"],
        "one catch-up run, not eight"
    );
}

#[test]
fn catchup_skip_records_the_missed_window_without_running_it() {
    let f = Fixture::healthy();
    f.write("rituals.md", RITUALS);
    let herdr = common::wire_herdr_split(
        &f,
        "\"{plan}\"",
        "\"{ritual}\"",
        "[scheduler]\ncatchup = \"skip\"\n\n",
        &["working", "idle"],
        None,
    );
    f.rituals_once(ANCHOR, &[]);
    let out = f.rituals_once("2026-10-05", &[]);

    assert_eq!(
        rituals_run(&herdr),
        vec!["conventions lint"],
        "nothing re-ran"
    );
    assert!(out.contains("window missed"), "{out}");
    assert_eq!(
        f.rituals_state()["runs"]["ritual:conventions lint"]["last_fired"],
        "2026-10-05",
        "recorded, so the next fire is a full period out rather than immediate"
    );
}

#[test]
fn a_cadence_with_no_day_count_never_fires_and_is_reported() {
    let (f, herdr) = fixture();
    f.rituals_once(ANCHOR, &[]);
    assert!(
        !rituals_run(&herdr).contains(&"incident review".to_string()),
        "'on demand' is not a schedule"
    );
    let state = f.rituals_state();
    assert!(state["runs"].get("ritual:incident review").is_none());
}

#[test]
fn a_dry_run_reports_the_command_and_changes_nothing() {
    let (f, herdr) = fixture();
    let out = f.rituals_once(ANCHOR, &["--dry-run"]);

    assert!(out.contains("would run ritual conventions lint"), "{out}");
    assert!(herdr.started_args().is_empty(), "nothing was spawned");
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
    // Only the socket: everything else stays at its default.
    let socket = f.root().join(".trellis/herdr.sock");
    let _herdr = FakeHerdr::start(socket.clone(), &["idle"], Vec::new());
    f.write("runtime.toml", &common::herdr_config(&socket));
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
    // No server needed: the placeholder is resolved before anything connects.
    f.write(
        "runtime.toml",
        "[harness.herdr]\nritual_args = [\"--plugin-dir\", \"{plugin_dir}\"]\n",
    );
    let out = f
        .bin()
        .args(["rituals"])
        .env_remove("CLAUDE_PLUGIN_ROOT")
        .env("HERDR_SOCKET_PATH", f.root().join(".trellis/no-herdr.sock"))
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
    let socket = f.root().join(".trellis/herdr.sock");
    let _herdr = FakeHerdr::start(socket.clone(), &["idle"], Vec::new());
    f.write(
        "runtime.toml",
        &format!(
            "[prompts]\nritual = \"/trellis:ritual {{ritual}}\"\n\n{}",
            common::herdr_config(&socket)
        ),
    );
    let out = f.rituals_once(ANCHOR, &["--dry-run"]);
    assert!(out.contains("⇐ /trellis:ritual conventions lint"), "{out}");
}

#[test]
fn a_domain_may_still_point_its_sessions_at_a_checkout() {
    let f = Fixture::healthy();
    f.write("rituals.md", RITUALS);
    let socket = f.root().join(".trellis/herdr.sock");
    let _herdr = FakeHerdr::start(socket.clone(), &["idle"], Vec::new());
    f.write(
        "runtime.toml",
        &format!(
            "{}ritual_args = [\"--plugin-dir\", \"{{plugin_dir}}\"]\n",
            common::herdr_config(&socket)
        ),
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
