//! The daemon's dispatcher: the same scan the forge workflow runs, with the
//! same hold and fail-closed semantics, now spawning the act sessions itself.
//! What makes it idempotent is not memory but the artifact — the plan goes
//! `ready → active` before its session starts, so the next scan no longer
//! sees it, whichever process is scanning (decision 0053).

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

/// Records argv, claims the plan, and leaves the verdict a taker owes —
/// `sh` run with `"$1"` naming the plan file.
fn verdict_fixture(verdict: &str) -> Fixture {
    let f = Fixture::healthy();
    let harness = f.fake_harness_leaving("verdict-harness", verdict);
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\"]\n\
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
    f.dispatch_once(ANCHOR, &[]);

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
    f.dispatch_once(ANCHOR, &[]);

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
    f.dispatch_once(ANCHOR, &["--map", "mechanical=haiku:low:1"]);
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
    f.dispatch_once(ANCHOR, &[]);
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
    let out = f.dispatch_once(ANCHOR, &[]);

    assert!(dispatched(&f).is_empty());
    assert!(out.contains("holding plans/waits.md"), "{out}");
    assert!(
        f.read("plans/waits.md").contains("status: ready"),
        "a hold is not a status change"
    );
}

#[test]
fn a_claimed_plan_leaves_the_queue_which_is_what_makes_dispatch_idempotent() {
    let f = verdict_fixture(
        "sed 's/^status: active$/status: retired/' \"$1\" > \"$1.v\" && mv \"$1.v\" \"$1\"",
    );
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    assert!(f.read("plans/ship-it.md").contains("status: retired"));

    // A new day, so the scan is due again — and finds nothing to do.
    f.dispatch_once("2026-08-04", &[]);
    assert_eq!(dispatched(&f).len(), 1, "not dispatched twice");
}

#[test]
fn a_session_that_ends_without_a_verdict_hands_its_plan_back_to_the_queue() {
    // The claiming harness is the failure this exists for: it takes the
    // plan and walks away. `active` with nobody holding it is a state no
    // scan reads and no tick retries — the plan would strand there.
    let f = claiming_fixture();
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    let out = f.dispatch_once(ANCHOR, &[]);

    assert_eq!(dispatched(&f), ["plans/ship-it.md"]);
    assert!(
        f.read("plans/ship-it.md").contains("status: ready"),
        "abandoned active plans go back to ready, not stranded:\n{out}"
    );
    assert!(out.contains("returned to ready"), "{out}");
}

#[test]
fn a_plan_parked_on_a_handoff_stays_active_and_out_of_the_queue() {
    // The taker left a proposal for its owner to rule on: the work is real,
    // it is simply not the runtime's to advance.
    let f = verdict_fixture(
        "awk '{print} /^status: active$/{print \"handoff: https://forge/pr/7\"}' \"$1\" > \"$1.v\" && mv \"$1.v\" \"$1\"",
    );
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    let out = f.dispatch_once(ANCHOR, &[]);

    let plan = f.read("plans/ship-it.md");
    assert!(plan.contains("status: active"), "{plan}");
    assert!(plan.contains("handoff: https://forge/pr/7"), "{plan}");
    assert!(out.contains("parked on https://forge/pr/7"), "{out}");
}

#[test]
fn a_ready_plan_with_no_owner_is_skipped_with_a_warning() {
    let f = claiming_fixture();
    f.write(
        "plans/ownerless.md",
        "---\nprovenance: authored\nstatus: ready\ntype: initiative\n---\n# Ownerless\n",
    );
    let out = f.dispatch_once(ANCHOR, &[]);

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
    let out = f.dispatch_once(ANCHOR, &[]);

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
    f.dispatch_once(ANCHOR, &[]);

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
    let out = f.dispatch_once(ANCHOR, &["--dry-run"]);

    assert!(
        out.contains("would run act plans/ship-it.md → org/founder"),
        "{out}"
    );
    assert!(f.invocations().is_empty());
    assert!(f.read("plans/ship-it.md").contains("status: ready"));
}

/// The tight loop's backstop (decision 0046): a session that ends without
/// claiming leaves its plan `ready`, and without a cooldown it would respawn
/// every tick, forever, at session prices.
#[test]
fn an_unclaimed_plan_cools_down_instead_of_respawning() {
    let f = passive_fixture(); // never claims
    f.write(
        "plans/loopy.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Loopy\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    assert_eq!(dispatched(&f).len(), 1);

    // Same day, moments later: the plan is still ready and its session is
    // gone — the cooldown, not the calendar, is what stops the respawn.
    let out = f.dispatch_once(ANCHOR, &[]);
    assert_eq!(dispatched(&f).len(), 1, "no respawn inside the cooldown");
    assert!(out.contains("cooling down"), "{out}");
}

#[test]
fn the_cooldown_is_the_bindings_knob() {
    let f = Fixture::healthy();
    let harness = f.fake_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[scheduler]\nretry_cooldown_secs = 0\n\n\
             [harness]\n\
             act_cmd = [\"{harness}\", \"{{owner}}\", \"{{plan}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\"]\n"
        ),
    );
    f.write(
        "plans/loopy.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Loopy\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    f.dispatch_once(ANCHOR, &[]);
    assert_eq!(
        dispatched(&f).len(),
        2,
        "cooldown zero: every pass may retry an unclaimed plan"
    );
}

/// The dispatch guard (decision 0053). Two dispatchers ran one plan twice
/// because the claim was the session's first turn, minutes after the spawn,
/// and the plan read `ready` throughout that window. The runtime takes it
/// before the session exists, so what the session finds is a plan already
/// claimed for it — and every scan, in any process, has left the queue.
#[test]
fn the_runtime_claims_the_plan_before_its_session_starts() {
    // This harness still claims, the way a session on the old contract
    // would. Its snapshot is taken first, so it reports what it was handed.
    let f = claiming_fixture();
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.dispatch_once(ANCHOR, &[]);

    let seen = f.plans_as_sessions_found_them();
    assert_eq!(seen.len(), 1, "one session, one snapshot: {seen:?}");
    assert!(
        seen[0].contains("status: active"),
        "the session found its plan already claimed: {:?}",
        seen[0]
    );
}

/// The claim is for a session that exists. A spawn that never got off the
/// ground leaves nothing holding the plan, and no retire will ever come to
/// undo it — so the claim comes back off here, or the plan strands `active`
/// with nobody on it, which is exactly the state decision 0052 closed.
#[test]
fn a_spawn_that_fails_puts_the_claim_back() {
    let f = Fixture::healthy();
    f.write(
        "runtime.toml",
        "[harness]\n\
         act_cmd = [\".trellis/bin/not-a-harness\", \"{plan}\"]\n\
         ritual_cmd = [\".trellis/bin/not-a-harness\", \"{ritual}\"]\n",
    );
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.dispatch_once(ANCHOR, &[]);

    let plan = std::fs::read_to_string(f.root().join("plans/ship-it.md")).unwrap();
    assert!(
        plan.contains("status: ready"),
        "a plan nothing is working on belongs in the queue: {plan:?}"
    );
}

/// `--once` skipped the lock on the reading that one pass is harmless. It is
/// not: a pass scans the same plans, spawns into the same slots, and writes
/// the same state file, so one-shot beside a daemon is the two-dispatchers
/// case under another name (decision 0053). The session re-enters the
/// binary, which is the only moment a live dispatcher exists to contend with.
#[test]
fn a_one_shot_pass_refuses_to_run_beside_a_live_dispatcher() {
    let f = Fixture::healthy();
    let bin = Fixture::bin_path();
    let harness = f.fake_harness_leaving(
        "reentrant",
        &format!(
            "\x20 \"{}\" dispatch run --once > \"$dir/../second-pass\" 2>&1\n",
            bin.display()
        ),
    );
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\"]\n"
        ),
    );
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.dispatch_once(ANCHOR, &[]);

    let refusal = std::fs::read_to_string(f.root().join(".trellis/runtime/second-pass"))
        .expect("the session ran a second pass");
    assert!(
        refusal.contains("already running"),
        "a one-shot pass under a live dispatcher is refused: {refusal:?}"
    );
    assert_eq!(
        dispatched(&f).len(),
        1,
        "and it dispatched nothing of its own"
    );
}

/// A dispatcher killed between the claim and the session's end leaves a plan
/// `active` that no retire will ever come to release — a wider window since
/// the claim moved ahead of the spawn (decision 0053). Startup already
/// reconciles the acting-role marker against what is actually in flight;
/// every line it drops is exactly such a plan, and it is handed back there.
#[test]
fn a_plan_whose_dispatcher_died_is_handed_back_at_startup() {
    let f = claiming_fixture();
    f.write(
        "plans/orphaned.md",
        &format!("{FM}status: active\ntype: initiative\n---\n# Orphaned\n"),
    );
    // What the dead dispatcher left behind: its marker line for a session
    // that no longer exists, and the state saying that session was an `act`.
    f.write(
        ".trellis/acting-role",
        "org/founder 2026-08-03T00:00:00Z plan:plans/orphaned.md\n",
    );
    f.write(
        ".trellis/runtime/dispatch-state.json",
        "{\"version\":2,\"runs\":{\"plan:plans/orphaned.md\":{\"last_errand\":\"act\"}}}\n",
    );

    let out = f.dispatch_once(ANCHOR, &[]);
    assert!(
        out.contains("plans/orphaned.md: its session ended with the plan still active"),
        "{out}"
    );
    assert_eq!(
        dispatched(&f),
        vec!["plans/orphaned.md"],
        "and the same pass picks it back up"
    );
}
