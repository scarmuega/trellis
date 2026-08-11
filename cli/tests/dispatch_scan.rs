mod common;

use common::Fixture;

fn scan(f: &Fixture, extra: &[&str]) -> serde_json::Value {
    let out = f
        .bin()
        .args(["dispatch", "scan", "--format", "json"])
        .args(extra)
        .output()
        .unwrap();
    assert!(out.status.success());
    serde_json::from_slice(&out.stdout).unwrap()
}

// Every dispatchable fixture declares a resolving subdomain: the scan now
// runs the mechanical readiness precheck (decision 0045), and a plan with no
// subdomains: fails item 4 and holds.
const FM: &str =
    "---\nprovenance: authored\nowner: org/founder\nsubdomains: [problem/outdoor-retail-channel.md]\n";

#[test]
fn scan_reproduces_the_awk_semantics() {
    let f = Fixture::healthy();
    // Dispatched: ready, no awaits, no complexity → standard session.
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    // Dispatched: deep tier.
    f.write(
        "plans/think-hard.md",
        &format!("{FM}status: ready\ntype: initiative\ncomplexity: deep\n---\n# Think\n"),
    );
    // Held: awaits an active plan (block-list form, like the awk parses).
    f.write(
        "plans/waits.md",
        &format!("{FM}status: ready\ntype: initiative\nawaits:\n  - plans/seasonal-recipe-line.md\n---\n# Waits\n"),
    );
    // Held + warning: awaits a plan that does not resolve (fail closed).
    f.write(
        "plans/ghost-dep.md",
        &format!("{FM}status: ready\ntype: initiative\nawaits: [plans/ghost.md]\n---\n# Ghost\n"),
    );
    // Released: awaits a retired plan.
    f.write(
        "plans/done-dep.md",
        &format!("{FM}status: retired\ntype: initiative\n---\n# Done\n"),
    );
    f.write(
        "plans/unblocked.md",
        &format!(
            "{FM}status: ready\ntype: initiative\nawaits: [plans/done-dep.md]\n---\n# Unblocked\n"
        ),
    );
    // Warning + skip: ready but ownerless.
    f.write(
        "plans/nobody.md",
        "---\nprovenance: authored\nstatus: ready\ntype: initiative\nsubdomains: [problem/outdoor-retail-channel.md]\n---\n# Nobody\n",
    );
    // Warning + standard: illegal complexity.
    f.write(
        "plans/weird.md",
        &format!("{FM}status: ready\ntype: initiative\ncomplexity: galactic\n---\n# Weird\n"),
    );
    // Not ready → invisible.
    f.write(
        "plans/drafty.md",
        &format!("{FM}status: draft\ntype: initiative\n---\n# Draft\n"),
    );

    let report = scan(&f, &[]);
    let dispatched: Vec<(&str, &str, &str, f64)> = report["dispatch"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["plan"].as_str().unwrap(),
                d["complexity"].as_str().unwrap(),
                d["model"].as_str().unwrap(),
                d["budget_usd"].as_f64().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        dispatched,
        vec![
            ("plans/ship-it.md", "standard", "opus", 10.0),
            ("plans/think-hard.md", "deep", "fable", 25.0),
            ("plans/unblocked.md", "standard", "opus", 10.0),
            ("plans/weird.md", "standard", "opus", 10.0),
        ],
        "{report:#}"
    );

    let held: Vec<(&str, &str)> = report["held"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| (h["plan"].as_str().unwrap(), h["awaits"].as_str().unwrap()))
        .collect();
    assert_eq!(
        held,
        vec![
            ("plans/ghost-dep.md", "plans/ghost.md"),
            ("plans/waits.md", "plans/seasonal-recipe-line.md"),
        ]
    );
    assert_eq!(
        report["held"][1]["target_status"], "active",
        "hold names the target's status"
    );

    let warnings = report["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 3, "{warnings:?}");
    assert!(warnings
        .iter()
        .any(|w| w.as_str().unwrap().contains("ghost")));
    assert!(warnings
        .iter()
        .any(|w| w.as_str().unwrap().contains("no owner")));
    assert!(warnings
        .iter()
        .any(|w| w.as_str().unwrap().contains("galactic")));
}

/// The mechanical readiness precheck (decision 0045): a plan the coder's
/// gate would bounce holds at the scan instead — no session spent
/// discovering it, nothing written, and the hold names the failed items.
#[test]
fn a_mechanically_unready_plan_holds_instead_of_dispatching() {
    let f = Fixture::healthy();
    // No subdomains: (item 4) and a deferral token in the body (item 7).
    f.write(
        "plans/vague.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: ready\ntype: initiative\n---\n# Vague\n\nTBD: pick one of the approaches.\n",
    );
    let report = scan(&f, &[]);
    assert!(
        report["dispatch"].as_array().unwrap().is_empty(),
        "{report:#}"
    );
    let h = &report["held"][0];
    assert_eq!(h["plan"], "plans/vague.md");
    assert_eq!(h["reason"], "readiness");
    let items: Vec<u64> = h["failed_items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i.as_u64().unwrap())
        .collect();
    assert!(items.contains(&4) && items.contains(&7), "{h:#}");
}

/// A holder ref declaring `kind: human` short-circuits into a handoff: the
/// scan reads one declared field and spawns nothing. Undeclared kinds keep
/// dispatching (the other tests' fixtures have no `kind:` and still spawn).
#[test]
fn a_declared_human_holder_gets_a_handoff_not_a_session() {
    let f = Fixture::healthy();
    f.write(
        "org/founder/holder/ref.md",
        "---\nprovenance: authored\nowner: org/founder\nref: santiago\nkind: human\n---\n# Founder holder\n",
    );
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );
    let report = scan(&f, &[]);
    assert!(
        report["dispatch"].as_array().unwrap().is_empty(),
        "{report:#}"
    );
    let h = &report["handoffs"][0];
    assert_eq!(h["plan"], "plans/ship-it.md");
    assert_eq!(h["owner"], "org/founder");
    assert_eq!(h["holder_ref"], "santiago");
}

/// The template defines metrics as rows of a definitions table and the plan
/// schema points `metrics:` refs at them — a resolver that only matched
/// headings failed the kernel's own convention, and with the precheck that
/// held every plan carrying a table-defined metric.
#[test]
fn a_metrics_ref_to_a_definitions_table_row_resolves() {
    let f = Fixture::healthy();
    f.write(
        "metrics/definitions.md",
        "---\nprovenance: authored\nowner: org/founder\n---\n# Metric definitions\n\n\
         | metric | definition | target | owner |\n\
         |--------|------------|--------|-------|\n\
         | doors-stocked | outfitter doors stocking kits | 40 | org/founder |\n",
    );
    f.write(
        "plans/tabled.md",
        &format!(
            "{FM}status: ready\ntype: initiative\nmetrics: [metrics/definitions.md#doors-stocked]\n---\n# Tabled\n"
        ),
    );
    let report = scan(&f, &[]);
    assert!(
        report["dispatch"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["plan"] == "plans/tabled.md"),
        "table-row metric ref dispatches, not held: {report:#}"
    );
}

#[test]
fn map_overrides_keep_the_workflow_the_tuning_point() {
    let f = Fixture::healthy();
    f.write(
        "plans/tuned.md",
        &format!("{FM}status: ready\ntype: initiative\ncomplexity: mechanical\n---\n# Tuned\n"),
    );
    let report = scan(&f, &["--map", "mechanical=haiku:low:1.5"]);
    let d = &report["dispatch"][0];
    assert_eq!(d["model"], "haiku");
    assert_eq!(d["effort"], "low");
    assert_eq!(d["budget_usd"], 1.5);
}
