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

const FM: &str = "---\nprovenance: authored\nowner: org/founder\n";

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
        "---\nprovenance: authored\nstatus: ready\ntype: initiative\n---\n# Nobody\n",
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
