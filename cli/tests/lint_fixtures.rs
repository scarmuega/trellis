mod common;

use common::{finding_items, Fixture};

#[test]
fn healthy_root_lints_clean() {
    let f = Fixture::healthy();
    let report = f.lint_json(&[]);
    assert_eq!(
        report["summary"]["violations"], 0,
        "healthy fixture must have zero violations: {report:#}"
    );
    assert_eq!(
        report["summary"]["warnings"], 0,
        "healthy fixture must have zero warnings: {report:#}"
    );
    let judged: Vec<u8> = report["judgment"]
        .as_array()
        .unwrap()
        .iter()
        .map(|j| j["item"].as_u64().unwrap() as u8)
        .collect();
    assert!(
        judged.contains(&7) && judged.contains(&16),
        "items 7 and 16 are always judgment: {judged:?}"
    );
    f.bin().arg("lint").assert().success();
}

#[test]
fn crafted_violations_fire_their_items() {
    let f = Fixture::healthy();

    // item 3 + 14: a subdomain with no induced-by edge is malformed and an orphan.
    f.write(
        "problem/unparented.md",
        "---\nprovenance: authored\nowner: org/founder\n---\n# Unparented\n",
    );
    // item 4: illegal status, unregistered type, illegal complexity,
    // self-await, missing await target.
    f.write(
        "plans/broken.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: cooking\ntype: moonshot\ncomplexity: galactic\nawaits: [plans/broken.md, plans/ghost.md]\n---\n# Broken\n",
    );
    // item 8: unregistered tag.
    f.write(
        "plans/tagged.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: draft\ntype: initiative\ntags: [skunkworks]\n---\n# Tagged\n",
    );
    // item 11: root-level skills/ and a plans/ subdirectory.
    f.write(
        "skills/how-to.md",
        "---\nprovenance: authored\nowner: org/founder\n---\n# How\n",
    );
    f.write("plans/family/sub.md", "---\nprovenance: authored\nowner: org/founder\nstatus: draft\ntype: initiative\n---\n# Sub\n");
    // item 20: a committed strategy with no sustaining edge.
    f.write(
        "strategy/unfunded.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: validated\nneed: market.md#n-trail-nutrition\ndifferentiation: cheaper than the incumbent\nfunded-by:\n  - strategy: strategy/dehydrated-meal-kits.md\n    relation: intended\n---\n# Unfunded\n",
    );
    // item 22: an awaits cycle.
    f.write(
        "plans/loop-a.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: ready\ntype: initiative\nawaits: [plans/loop-b.md]\n---\n# A\n",
    );
    f.write(
        "plans/loop-b.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: ready\ntype: initiative\nawaits: [plans/loop-a.md]\n---\n# B\n",
    );
    // item 24: blocked with no open record; and a record in a generated artifact.
    f.write(
        "plans/stuck.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: blocked\ntype: initiative\n---\n# Stuck\n",
    );
    // item 1: an artifact with no frontmatter at all.
    f.write("problem/bare.md", "# Bare\n");

    let report = f.lint_json(&[]);
    let items = finding_items(&report);
    for expected in [1u8, 3, 4, 8, 11, 14, 20, 22, 24] {
        assert!(
            items.contains(&expected),
            "expected item {expected} to fire; got {items:?}\n{report:#}"
        );
    }
    f.bin().arg("lint").assert().code(1);
}

#[test]
fn blocked_with_open_record_satisfies_item_24() {
    let f = Fixture::healthy();
    f.write(
        "plans/stuck-but-stated.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: blocked\ntype: initiative\nsubdomains: [problem/recipe-development.md]\ncontexts: [solution/kit-kitchen]\n---\n# Stuck but stated\n\n## Escalations\n\n```yaml\nraised: %%DAYS_AGO_1%%\nby: org/founder\nto: org/founder\nstatus: open\nasks: which supplier contract do we accept?\n```\n",
    );
    let report = f.lint_json(&[]);
    let item24: Vec<_> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["item"] == 24)
        .collect();
    assert!(
        item24.is_empty(),
        "well-formed blocked plan must not fire item 24: {item24:?}"
    );
}

#[test]
fn accepted_decision_edited_after_acceptance_fires_item_9() {
    let f = Fixture::healthy();
    // The composing commit already holds 0000 as accepted. Edit + commit.
    let text = f.read("decisions/0000-adopt-trellis.md");
    f.write(
        "decisions/0000-adopt-trellis.md",
        &format!("{text}\nRetroactive tweak.\n"),
    );
    f.commit_at("2026-07-20", "tweak accepted decision");
    let report = f.lint_json(&["--items", "9"]);
    assert!(
        finding_items(&report).contains(&9),
        "post-acceptance edit must fire item 9: {report:#}"
    );
}

#[test]
fn spec_version_pin_mismatch_fires_item_18() {
    let f = Fixture::healthy();
    let text = f
        .read("decisions/0000-adopt-trellis.md")
        .replace("v16", "v15");
    f.write("decisions/0000-adopt-trellis.md", &text);
    let report = f.lint_json(&["--items", "18"]);
    let messages: Vec<String> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect();
    assert!(
        messages.iter().any(|m| m.contains("v15")),
        "stale pin must fire item 18: {report:#}"
    );
}

#[test]
fn stale_actuals_fire_item_12_once_a_window_exists() {
    let f = Fixture::healthy();
    // The healthy fixture ships no rituals.md — item 12 must stay silent.
    let report = f.lint_json(&["--items", "12"]);
    assert_eq!(report["summary"]["violations"], 0);

    // Declare the heartbeat; actuals from 2 days ago sit inside 7 days.
    f.write(
        "rituals.md",
        "---\nprovenance: authored\nowner: org/founder\n---\n# Rituals\n\n| ritual | cadence | executor | procedure |\n|---|---|---|---|\n| metric sweep | weekly | org/founder | diff actuals vs targets |\n| plan dispatch | daily | org/founder | scan ready plans |\n",
    );
    let report = f.lint_json(&["--items", "12"]);
    assert_eq!(report["summary"]["violations"], 0, "{report:#}");

    // Age the actuals out of the window.
    let stale = f.read("metrics/actuals/latest.md");
    let dated = regex::Regex::new(r"date: \d{4}-\d{2}-\d{2}")
        .unwrap()
        .replace(&stale, "date: 2026-06-30")
        .to_string();
    f.write("metrics/actuals/latest.md", &dated);
    let report = f.lint_json(&["--items", "12"]);
    assert!(finding_items(&report).contains(&12), "{report:#}");
}

#[test]
fn owner_escalation_shape_travels_in_findings() {
    let f = Fixture::healthy();
    f.write("problem/bare.md", "# Bare\n");
    let report = f.lint_json(&["--items", "1"]);
    let finding = &report["findings"][0];
    assert_eq!(finding["path"], "problem/bare.md");
    assert!(finding["message"].as_str().unwrap().contains("provenance"));
}
