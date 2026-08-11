//! The daemon's dispatcher: the same scan the forge workflow runs, with the
//! same hold and fail-closed semantics, now spawning the act sessions itself.
//! What makes it idempotent is not memory but the artifact — a taker flips
//! its plan `ready → active`, so the next scan no longer sees it.

mod common;

use common::{Fixture, ANCHOR};

// The subdomain keeps ready fixtures past the scan's mechanical readiness
// precheck (decision 0045).
const FM: &str =
    "---\nprovenance: authored\nowner: org/founder\nsubdomains: [problem/outdoor-retail-channel.md]\n";

/// Records argv and claims the plan named in its first argument.
fn claiming_fixture() -> Fixture {
    let f = Fixture::healthy();
    let harness = f.fake_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\", \"{{owner}}\", \"{{complexity}}\", \
             \"{{model}}\", \"{{effort}}\", \"{{budget}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\"]\n"
        ),
    );
    f
}

/// Records argv and leaves every plan exactly as it found it.
fn passive_fixture() -> Fixture {
    let f = Fixture::healthy();
    let harness = f.fake_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[scheduler]\nmax_concurrent = 1\n\n\
             [harness]\n\
             act_cmd = [\"{harness}\", \"{{owner}}\", \"{{plan}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\"]\n"
        ),
    );
    f
}

/// Which plans got a session, sorted — concurrent sessions finish in
/// whatever order they finish, and the daemon promises nothing about it.
fn dispatched(f: &Fixture) -> Vec<String> {
    let mut plans: Vec<String> = f
        .invocations()
        .into_iter()
        .filter_map(|argv| argv.into_iter().find(|a| a.starts_with("plans/")))
        .collect();
    plans.sort();
    plans
}

/// The argv of the session that ran one plan.
fn invocation_for(f: &Fixture, plan: &str) -> Vec<String> {
    f.invocations()
        .into_iter()
        .find(|argv| argv.first().map(String::as_str) == Some(plan))
        .unwrap_or_else(|| panic!("no session ran {plan}; saw {:?}", f.invocations()))
}

#[test]
fn every_ready_plan_gets_one_session_sized_by_its_complexity() {
    let f = claiming_fixture();
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.write(
        "plans/think-hard.md",
        &format!("{FM}status: ready\ntype: initiative\ncomplexity: deep\n---\n# Think\n"),
    );
    f.serve_once(ANCHOR, &[]);

    assert_eq!(
        dispatched(&f),
        vec!["plans/ship-it.md", "plans/think-hard.md"]
    );
    assert_eq!(
        invocation_for(&f, "plans/ship-it.md"),
        vec![
            "plans/ship-it.md",
            "founder",
            "standard",
            "opus",
            "high",
            "10"
        ],
        "an unmarked plan takes the standard session"
    );
    assert_eq!(
        &invocation_for(&f, "plans/think-hard.md")[2..],
        ["deep", "fable", "xhigh", "25"],
        "the deep tier gets the deep session"
    );
}

/// The runtime stamps `.trellis/acting-role` around every session it spawns
/// (decision 0045): present — with the acting role — while the session runs,
/// gone when the last session retires. The fake harness snapshots the marker
/// from inside its run, which is the only moment it can be seen.
#[test]
fn the_daemon_stamps_the_acting_role_marker_around_each_session() {
    let f = claiming_fixture();
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.serve_once(ANCHOR, &[]);

    let seen: Vec<_> = std::fs::read_dir(f.root().join(".trellis/runtime"))
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("marker-seen"))
        .collect();
    assert!(!seen.is_empty(), "a session saw the marker while it ran");
    let content = std::fs::read_to_string(seen[0].path()).unwrap();
    assert!(
        content
            .lines()
            .any(|l| l.starts_with("org/founder ") && l.ends_with(" plan:plans/ship-it.md")),
        "the marker names the acting role and the session: {content:?}"
    );
    assert!(
        !f.root().join(".trellis/acting-role").exists(),
        "the marker leaves with the last session"
    );
}

#[test]
fn the_session_map_is_the_bindings_tuning_point() {
    let f = claiming_fixture();
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\ncomplexity: mechanical\n---\n# Ship\n"),
    );
    f.serve_once(ANCHOR, &["--map", "mechanical=haiku:low:1"]);
    assert_eq!(&f.invocations()[0][3..], ["haiku", "low", "1"]);
}

#[test]
fn sessions_configured_in_runtime_toml_override_the_defaults() {
    let f = Fixture::healthy();
    let harness = f.fake_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[sessions]\nstandard = \"sonnet:medium:2\"\n\n\
             [harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\", \"{{model}}\", \"{{effort}}\", \"{{budget}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\"]\n"
        ),
    );
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.serve_once(ANCHOR, &[]);
    assert_eq!(&f.invocations()[0][1..], ["sonnet", "medium", "2"]);
}

#[test]
fn a_held_plan_stays_ready_and_is_never_dispatched() {
    let f = claiming_fixture();
    f.write(
        "plans/waits.md",
        &format!(
            "{FM}status: ready\ntype: initiative\nawaits: [plans/seasonal-recipe-line.md]\n---\n# Waits\n"
        ),
    );
    let out = f.serve_once(ANCHOR, &[]);

    assert!(dispatched(&f).is_empty());
    assert!(out.contains("holding plans/waits.md"), "{out}");
    assert!(
        f.read("plans/waits.md").contains("status: ready"),
        "a hold is not a status change"
    );
}

#[test]
fn a_claimed_plan_leaves_the_queue_which_is_what_makes_dispatch_idempotent() {
    let f = claiming_fixture();
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.serve_once(ANCHOR, &[]);
    assert!(f.read("plans/ship-it.md").contains("status: active"));

    // A new day, so the scan is due again — and finds nothing to do.
    f.serve_once("2026-08-04", &[]);
    assert_eq!(dispatched(&f).len(), 1, "not dispatched twice");
}

#[test]
fn a_ready_plan_with_no_owner_is_skipped_with_a_warning() {
    let f = claiming_fixture();
    f.write(
        "plans/ownerless.md",
        "---\nprovenance: authored\nstatus: ready\ntype: initiative\n---\n# Ownerless\n",
    );
    let out = f.serve_once(ANCHOR, &[]);

    assert!(dispatched(&f).is_empty());
    assert!(out.contains("declares no owner"), "{out}");
}

#[test]
fn the_concurrency_cap_defers_work_and_one_pass_drains_it() {
    let f = passive_fixture(); // one slot, and a harness that claims nothing
    for slug in ["alpha", "beta"] {
        f.write(
            &format!("plans/{slug}.md"),
            &format!("{FM}status: ready\ntype: initiative\n---\n# {slug}\n"),
        );
    }
    let out = f.serve_once(ANCHOR, &[]);

    assert!(
        out.contains("session slots busy"),
        "the cap was reached: {out}"
    );
    assert_eq!(
        dispatched(&f),
        vec!["plans/alpha.md", "plans/beta.md"],
        "and --once kept passing until the due set was drained — a cron entry \
         must not leave a day's work unstarted because the cap is two"
    );
}

#[test]
fn a_drained_pass_never_exceeds_the_cap_at_once() {
    let f = passive_fixture();
    // A harness that fails if a sibling is already running: the marker is
    // taken for the life of the session and released on exit.
    std::fs::write(
        f.root().join(".trellis/bin/harness"),
        "#!/bin/sh\n\
         dir=.trellis/runtime/argv\n\
         mkdir -p \"$dir\"\n\
         if ! mkdir .trellis/runtime/slot 2>/dev/null; then\n\
         \x20 echo overlapped > \"$dir/violation-$$\"; exit 1\n\
         fi\n\
         for a in \"$@\"; do echo \"$a\"; done > \"$dir/$(date +%s)-$$\"\n\
         sleep 1\n\
         rmdir .trellis/runtime/slot\n\
         exit 0\n",
    )
    .unwrap();
    for slug in ["alpha", "beta", "gamma"] {
        f.write(
            &format!("plans/{slug}.md"),
            &format!("{FM}status: ready\ntype: initiative\n---\n# {slug}\n"),
        );
    }
    f.serve_once(ANCHOR, &[]);

    assert_eq!(dispatched(&f).len(), 3, "every plan ran");
    assert!(
        f.invocations().iter().all(|argv| argv != &["overlapped"]),
        "draining runs passes back to back, never two sessions in one slot"
    );
}

#[test]
fn a_dry_run_reports_the_dispatch_without_starting_it() {
    let f = claiming_fixture();
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    let out = f.serve_once(ANCHOR, &["--dry-run"]);

    assert!(
        out.contains("would run act plans/ship-it.md → org/founder"),
        "{out}"
    );
    assert!(f.invocations().is_empty());
    assert!(f.read("plans/ship-it.md").contains("status: ready"));
}
