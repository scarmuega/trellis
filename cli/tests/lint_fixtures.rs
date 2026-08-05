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
fn git_ignored_trees_are_not_artifacts() {
    let f = Fixture::healthy();
    // A deployment unit with its dependencies installed — the shape
    // spec/model.md puts inside solution/{bc}/ by design.
    f.write(".gitignore", "node_modules/\n");
    f.write(
        "solution/kit-kitchen/storefront/node_modules/left-pad/README.md",
        "# left-pad\n\nNo frontmatter, and none of the domain's business.\n",
    );
    let report = f.lint_json(&[]);
    assert_eq!(
        report["summary"]["violations"], 0,
        "a git-ignored dependency tree is not the domain's markdown: {report:#}"
    );
    assert!(
        !report["scope"]["git_ignored"]
            .as_array()
            .unwrap()
            .is_empty(),
        "the exclusion must be reported, never silent: {report:#}"
    );

    // The same tree once the deployment unit is committed: git reports an
    // ignored directory differently depending on whether its parent is
    // tracked, and both readings must prune.
    f.commit_at("2026-07-01", "commit the deployment unit");
    let report = f.lint_json(&[]);
    assert_eq!(
        report["summary"]["violations"], 0,
        "committing the parent must not resurrect the dependency tree: {report:#}"
    );
}

#[test]
fn a_nested_repository_is_never_this_domains_markdown() {
    let f = Fixture::healthy();

    // A submodule as `conventions.md`'s boundary guarantees sanction it: a
    // gitlink in the index, so no ignore query will ever name it, and a
    // `.git` *file* on disk.
    let src = tempfile::TempDir::new().unwrap();
    let upstream = src.path().join("upstream");
    std::fs::create_dir_all(upstream.join("docs")).unwrap();
    std::fs::write(upstream.join("README.md"), "# Upstream\n").unwrap();
    std::fs::write(upstream.join("docs/guide.md"), "# Guide\n").unwrap();
    for args in [
        vec!["init", "-q", "."],
        vec!["config", "user.email", "up@example.invalid"],
        vec!["config", "user.name", "upstream"],
        vec!["add", "-A"],
        vec!["commit", "-q", "-m", "upstream"],
    ] {
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&upstream)
            .args(&args)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
    }
    let url = upstream.to_str().unwrap();
    f.git(&[
        "-c",
        "protocol.file.allow=always",
        "-c",
        "user.email=fixture@example.invalid",
        "-c",
        "user.name=fixture",
        "submodule",
        "add",
        "-q",
        url,
        "solution/kit-kitchen/vendor/upstream",
    ]);

    // And a stray clone nobody registered — a `.git` directory, equally
    // invisible to every ignore query.
    f.git(&[
        "-c",
        "protocol.file.allow=always",
        "clone",
        "-q",
        url,
        "solution/kit-kitchen/stray",
    ]);

    let report = f.lint_json(&[]);
    let paths: Vec<&str> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert!(
        !paths
            .iter()
            .any(|p| p.contains("vendor/upstream") || p.contains("stray")),
        "another repository's markdown is never this domain's: {paths:?}"
    );

    let nested: Vec<&str> = report["scope"]["nested_repos"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap())
        .collect();
    assert_eq!(
        nested,
        vec![
            "solution/kit-kitchen/stray",
            "solution/kit-kitchen/vendor/upstream",
        ],
        "both are reported, so the narrowing is never silent: {report:#}"
    );
}

#[test]
fn carried_content_is_not_an_artifact_but_is_still_swept() {
    let f = Fixture::healthy();
    // Tracked code inside a bounded context: markdown that belongs to the
    // app, plus a file carrying something the secrets policy forbids.
    f.write(
        "solution/kit-kitchen/storefront/README.md",
        "# Storefront\n\nThe app's own README.\n",
    );
    f.write(
        "solution/kit-kitchen/storefront/deploy.md",
        "# Deploy\n\n    aws_access_key_id = AKIAIOSFODNN7EXAMPLE\n",
    );

    // Control: undeclared, it is an artifact like any other and fails item 1.
    let report = f.lint_json(&[]);
    let paths: Vec<&str> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["item"] == 1)
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert!(
        paths.contains(&"solution/kit-kitchen/storefront/README.md"),
        "undeclared markdown is still an artifact: {report:#}"
    );

    // Declare it carried.
    let conv = f.read("conventions.md");
    f.write(
        "conventions.md",
        &format!(
            "{conv}\n## Carried-content registry\n\n\
             - `solution/kit-kitchen/storefront/` — the storefront app's own tree\n"
        ),
    );

    let report = f.lint_json(&[]);
    let item1: Vec<&str> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["item"] == 1)
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert!(
        item1.is_empty(),
        "declared carried markdown is not an artifact: {item1:?}"
    );
    assert_eq!(
        report["scope"]["carried"][0], "solution/kit-kitchen/storefront/",
        "{report:#}"
    );

    // But it is tracked, so the secrets sweep still reaches it.
    let item10: Vec<&str> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["item"] == 10)
        .map(|f| f["path"].as_str().unwrap())
        .collect();
    assert_eq!(
        item10,
        vec!["solution/kit-kitchen/storefront/deploy.md"],
        "carried content leaves item 10's reach untouched: {report:#}"
    );
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
