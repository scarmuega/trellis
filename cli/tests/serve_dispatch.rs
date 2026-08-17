//! The daemon's dispatcher: the same scan the forge workflow runs, with the
//! same hold and fail-closed semantics, now spawning the act sessions itself.
//! What makes it idempotent is not memory but the artifact — the plan goes
//! `ready → active` before its session starts, so the next scan no longer
//! sees it, whichever process is scanning (decision 0053).

mod common;

use common::{FakeHerdr, Fixture, ANCHOR};

// The subdomain keeps ready fixtures past the scan's mechanical readiness
// precheck (decision 0045).
const FM: &str =
    "---\nprovenance: authored\nowner: org/founder\nsubdomains: [problem/outdoor-retail-channel.md]\n";

/// A herdr-backed fixture whose session leaves `verdict` — or, with `None`,
/// claims its plan and walks away, which is the case the recycle rule exists
/// for. `args` is the flag array, the placeholder surface these tests read.
fn herdr_fixture(
    args: &str,
    extra: &str,
    verdict: Option<(&str, Option<&str>)>,
    statuses: &[&str],
) -> (Fixture, FakeHerdr) {
    let f = Fixture::healthy();
    let socket = f.root().join(".trellis/herdr.sock");
    let herdr = FakeHerdr::start_acting(
        socket.clone(),
        statuses,
        f.root(),
        common::session(f.root(), verdict),
    );
    f.write(
        "runtime.toml",
        &format!(
            "{extra}{}act_args = [{args}]\nritual_args = [\"{{ritual}}\"]\n",
            common::herdr_config(&socket)
        ),
    );
    (f, herdr)
}

/// Renders the placeholders a session is sized by, and leaves no verdict.
fn claiming_fixture() -> (Fixture, FakeHerdr) {
    herdr_fixture(
        "\"{plan}\", \"{owner}\", \"{complexity}\", \"{model}\", \"{effort}\"",
        "",
        None,
        &["working", "idle"],
    )
}

/// Leaves the verdict a taker owes its plan.
fn verdict_fixture(status: &str, handoff: Option<&str>) -> (Fixture, FakeHerdr) {
    herdr_fixture(
        "\"{plan}\"",
        "",
        Some((status, handoff)),
        &["working", "idle"],
    )
}

/// One slot, and every plan left exactly as it was found.
fn passive_fixture() -> (Fixture, FakeHerdr) {
    herdr_fixture(
        "\"{owner}\", \"{plan}\"",
        "[scheduler]\nmax_concurrent = 1\n\n",
        None,
        &["working", "idle"],
    )
}

/// Which plans got a session, sorted — concurrent sessions finish in
/// whatever order they finish, and the daemon promises nothing about it.
fn dispatched(h: &FakeHerdr) -> Vec<String> {
    let mut plans: Vec<String> = h
        .started_args()
        .into_iter()
        .filter_map(|args| args.into_iter().find(|a| a.starts_with("plans/")))
        .collect();
    plans.sort();
    plans
}

/// The flags of the session that ran one plan.
fn invocation_for(h: &FakeHerdr, plan: &str) -> Vec<String> {
    h.started_args()
        .into_iter()
        .find(|args| args.first().map(String::as_str) == Some(plan))
        .unwrap_or_else(|| panic!("no session for {plan}"))
}

#[test]
fn every_ready_plan_gets_one_session_sized_by_its_complexity() {
    let (f, herdr) = claiming_fixture();
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
        dispatched(&herdr),
        vec!["plans/ship-it.md", "plans/think-hard.md"]
    );
    assert_eq!(
        invocation_for(&herdr, "plans/ship-it.md"),
        vec!["plans/ship-it.md", "founder", "standard", "opus", "high"],
        "an unmarked plan takes the standard session"
    );
    assert_eq!(
        &invocation_for(&herdr, "plans/think-hard.md")[2..],
        ["deep", "fable", "xhigh"],
        "the deep tier gets the deep session"
    );
}

/// The runtime stamps `.trellis/acting-role` around every session it spawns
/// (decision 0045): present — with the acting role — while the session runs,
/// gone when the last session retires. The fake harness snapshots the marker
/// from inside its run, which is the only moment it can be seen.
#[test]
fn the_daemon_stamps_the_acting_role_marker_around_each_session() {
    let (f, _herdr) = claiming_fixture();
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
    let (f, herdr) = claiming_fixture();
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\ncomplexity: mechanical\n---\n# Ship\n"),
    );
    f.dispatch_once(ANCHOR, &["--map", "mechanical=haiku:low:1"]);
    assert_eq!(&herdr.started_args()[0][3..], ["haiku", "low"]);
}

#[test]
fn sessions_configured_in_runtime_toml_override_the_defaults() {
    let (f, herdr) = herdr_fixture(
        "\"{plan}\", \"{model}\", \"{effort}\"",
        "[sessions]\nstandard = \"sonnet:medium:2\"\n\n",
        None,
        &["working", "idle"],
    );
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    assert_eq!(&herdr.started_args()[0][1..], ["sonnet", "medium"]);
}

#[test]
fn a_held_plan_stays_ready_and_is_never_dispatched() {
    let (f, herdr) = claiming_fixture();
    f.write(
        "plans/waits.md",
        &format!(
            "{FM}status: ready\ntype: initiative\nawaits: [plans/seasonal-recipe-line.md]\n---\n# Waits\n"
        ),
    );
    let out = f.dispatch_once(ANCHOR, &[]);

    assert!(dispatched(&herdr).is_empty());
    assert!(out.contains("holding plans/waits.md"), "{out}");
    assert!(
        f.read("plans/waits.md").contains("status: ready"),
        "a hold is not a status change"
    );
}

#[test]
fn a_claimed_plan_leaves_the_queue_which_is_what_makes_dispatch_idempotent() {
    let (f, herdr) = verdict_fixture("retired", None);
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    assert!(f.read("plans/ship-it.md").contains("status: retired"));

    // A new day, so the scan is due again — and finds nothing to do.
    f.dispatch_once("2026-08-04", &[]);
    assert_eq!(dispatched(&herdr).len(), 1, "not dispatched twice");
}

#[test]
fn a_session_that_ends_without_a_verdict_hands_its_plan_back_to_the_queue() {
    // The claiming harness is the failure this exists for: it takes the
    // plan and walks away. `active` with nobody holding it is a state no
    // scan reads and no tick retries — the plan would strand there.
    let (f, herdr) = claiming_fixture();
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    let out = f.dispatch_once(ANCHOR, &[]);

    assert_eq!(dispatched(&herdr), ["plans/ship-it.md"]);
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
    let (f, _herdr) = verdict_fixture("active", Some("https://forge/pr/7"));
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
    let (f, herdr) = claiming_fixture();
    f.write(
        "plans/ownerless.md",
        "---\nprovenance: authored\nstatus: ready\ntype: initiative\n---\n# Ownerless\n",
    );
    let out = f.dispatch_once(ANCHOR, &[]);

    assert!(dispatched(&herdr).is_empty());
    assert!(out.contains("declares no owner"), "{out}");
}

#[test]
fn the_concurrency_cap_defers_work_and_one_pass_drains_it() {
    let (f, herdr) = passive_fixture(); // one slot, and a harness that claims nothing
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
        dispatched(&herdr),
        vec!["plans/alpha.md", "plans/beta.md"],
        "and --once kept passing until the due set was drained — a cron entry \
         must not leave a day's work unstarted because the cap is two"
    );
}

#[test]
fn a_drained_pass_never_exceeds_the_cap_at_once() {
    // `retain = "never"` so every retirement closes its workspace, which
    // makes the concurrency readable straight off the wire.
    let (f, herdr) = herdr_fixture(
        "\"{plan}\"",
        "[scheduler]\nmax_concurrent = 1\n\n",
        Some(("retired", None)),
        &["working", "idle"],
    );
    f.write("runtime.toml", &{
        let mut t = f.read("runtime.toml");
        t.push_str("retain = \"never\"\n");
        t
    });
    for slug in ["alpha", "beta", "gamma"] {
        f.write(
            &format!("plans/{slug}.md"),
            &format!("{FM}status: ready\ntype: initiative\n---\n# {slug}\n"),
        );
    }
    f.dispatch_once(ANCHOR, &[]);

    assert_eq!(dispatched(&herdr).len(), 3, "every plan ran");
    assert_eq!(
        herdr.peak_open(),
        1,
        "draining runs passes back to back, never two sessions in one slot"
    );
}

#[test]
fn a_dry_run_reports_the_dispatch_without_starting_it() {
    let (f, _herdr) = claiming_fixture();
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
    let (f, herdr) = passive_fixture(); // never claims
    f.write(
        "plans/loopy.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Loopy\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    assert_eq!(dispatched(&herdr).len(), 1);

    // Same day, moments later: the plan is still ready and its session is
    // gone — the cooldown, not the calendar, is what stops the respawn.
    let out = f.dispatch_once(ANCHOR, &[]);
    assert_eq!(
        dispatched(&herdr).len(),
        1,
        "no respawn inside the cooldown"
    );
    assert!(out.contains("cooling down"), "{out}");
}

#[test]
fn the_cooldown_is_the_bindings_knob() {
    let (f, herdr) = herdr_fixture(
        "\"{owner}\", \"{plan}\"",
        "[scheduler]\nretry_cooldown_secs = 0\n\n",
        None,
        &["working", "idle"],
    );
    f.write(
        "plans/loopy.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Loopy\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    f.dispatch_once(ANCHOR, &[]);
    assert_eq!(
        dispatched(&herdr).len(),
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
    let (f, _herdr) = claiming_fixture();
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
    let socket = f.root().join(".trellis/herdr.sock");
    // A pane that never becomes a session: herdr takes the workspace and
    // then refuses to start the agent in it.
    let _herdr = FakeHerdr::start_refusing(socket.clone(), "agent_unsupported");
    f.write(
        "runtime.toml",
        &format!(
            "{}act_args = [\"{{plan}}\"]\n",
            common::herdr_config(&socket)
        ),
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
    assert!(
        !f.root().join(".trellis/acting-role").exists(),
        "and the marker goes back with the claim"
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
    let socket = f.root().join(".trellis/herdr.sock");
    // The second pass runs while the first is mid-spawn — the only moment a
    // live dispatcher exists to contend with.
    let root = f.root().to_path_buf();
    let herdr = FakeHerdr::start_acting(
        socket.clone(),
        &["working", "idle"],
        f.root(),
        Box::new(move |_: &str, _: &std::path::Path| {
            let out = std::process::Command::new(&bin)
                .arg("dispatch")
                .arg("run")
                .arg("--once")
                .current_dir(&root)
                .env("TRELLIS_TODAY", ANCHOR)
                .output()
                .unwrap();
            let mut text = String::from_utf8_lossy(&out.stdout).to_string();
            text.push_str(&String::from_utf8_lossy(&out.stderr));
            std::fs::write(root.join(".trellis/runtime/second-pass"), text).unwrap();
        }),
    );
    f.write(
        "runtime.toml",
        &format!(
            "{}act_args = [\"{{plan}}\"]\n",
            common::herdr_config(&socket)
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
        dispatched(&herdr).len(),
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
    let (f, herdr) = claiming_fixture();
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
        dispatched(&herdr),
        vec!["plans/orphaned.md"],
        "and the same pass picks it back up"
    );
}

/// The budget is still computed and still rendered — it simply cannot be an
/// argument, because an interactive session has nothing to enforce it with
/// (decision 0043). A binding that wants the session to know its ceiling
/// says so in the prompt, and that is all the number is now: a statement.
#[test]
fn the_budget_reaches_the_prompt_even_though_it_can_never_be_a_flag() {
    let (f, herdr) = herdr_fixture(
        "\"{plan}\"",
        "[prompts]\nact = \"{plan} budget={budget} tier={complexity}\"\n\n",
        None,
        &["working", "idle"],
    );
    f.write(
        "plans/think-hard.md",
        &format!("{FM}status: ready\ntype: initiative\ncomplexity: deep\n---\n# Think\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    assert_eq!(herdr.prompts(), ["plans/think-hard.md budget=25 tier=deep"]);
}

/// The recycle is not a dead end: the plan goes back to `ready`, the
/// cooldown paces the retry, and the next attempt is a *fresh* pane. The
/// first pane is left open on purpose (`retain = "on-failure"`) — it is the
/// scene of the stall, and an operator may want to read it.
#[test]
fn a_recycled_plan_is_retried_in_a_pane_of_its_own() {
    let (f, herdr) = herdr_fixture(
        "\"{plan}\"",
        "[scheduler]\nretry_cooldown_secs = 0\n\n",
        None,
        &["working", "idle"],
    );
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    f.dispatch_once(ANCHOR, &[]);

    assert_eq!(
        dispatched(&herdr).len(),
        2,
        "the plan came back and went out again"
    );
    let panes: Vec<String> = herdr
        .params_for("agent.start")
        .iter()
        .map(|p| p["pane_id"].as_str().unwrap().to_string())
        .collect();
    assert_ne!(panes[0], panes[1], "a retry is a new session, not a nudge");
    assert!(
        !herdr.calls().iter().any(|c| c == "workspace.close"),
        "and the stalled pane stays up for attach"
    );
}

/// The cooldown is paced from the conclusion, not the spawn. A session that
/// stalls for a long time and is then recycled must not be re-dispatched by
/// the very next pass just because its attempt stamp is old.
#[test]
fn the_cooldown_runs_from_the_recycle_not_the_spawn() {
    let (f, herdr) = herdr_fixture(
        "\"{plan}\"",
        "[scheduler]\nretry_cooldown_secs = 900\n\n",
        None,
        &["working", "idle"],
    );
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    f.dispatch_once(ANCHOR, &[]);
    // The plan is back in the queue and readable as ready…
    assert!(f.read("plans/ship-it.md").contains("status: ready"));
    // …and still not re-dispatched, because the recycle restarted the clock.
    let out = f.dispatch_once(ANCHOR, &[]);
    assert_eq!(dispatched(&herdr).len(), 1, "{out}");
    assert!(out.contains("cooling down"), "{out}");
}
