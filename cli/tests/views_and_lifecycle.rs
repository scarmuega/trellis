mod common;

use common::{finding_items, Fixture};

#[test]
fn board_is_deterministic_and_censuses_the_plans() {
    let f = Fixture::healthy();
    let a = f.bin().args(["view", "board"]).output().unwrap();
    let b = f.bin().args(["view", "board"]).output().unwrap();
    assert!(a.status.success());
    assert_eq!(
        a.stdout, b.stdout,
        "same tree + HEAD + day ⇒ byte-identical"
    );
    let text = String::from_utf8(a.stdout).unwrap();
    assert!(
        text.starts_with("---\nprovenance: generated\n"),
        "views stamp provenance"
    );
    assert!(text.contains("generated-by: trellis view board"));
    assert!(
        text.contains("| active | 2 |"),
        "healthy fixture has two active plans:\n{text}"
    );
    assert!(
        !text.to_lowercase().contains("| progress"),
        "no progress column, ever (decision 0030)"
    );
}

#[test]
fn written_board_satisfies_item_2_until_hand_edited() {
    let f = Fixture::healthy();
    f.bin()
        .args(["view", "board", "--write"])
        .assert()
        .success();
    f.commit_at("2026-08-03", "steward: refresh board");

    let report = f.lint_json(&["--items", "2"]);
    assert_eq!(
        report["summary"]["violations"], 0,
        "fresh board diffs clean: {report:#}"
    );
    let checked = &report["judgment"][0]["checked_mechanically"];
    assert_eq!(checked[0], "metrics/actuals/plan-board.md");

    f.bin()
        .args(["view", "board", "--check"])
        .assert()
        .success();

    // Hand-edit → item 2 and --check both catch it.
    let board = f.read("metrics/actuals/plan-board.md");
    f.write(
        "metrics/actuals/plan-board.md",
        &format!("{board}\nA hopeful hand-written line.\n"),
    );
    let report = f.lint_json(&["--items", "2"]);
    assert!(finding_items(&report).contains(&2), "{report:#}");
    f.bin().args(["view", "board", "--check"]).assert().code(1);
}

#[test]
fn escalations_view_lists_open_records() {
    let f = Fixture::healthy();
    let out = f.bin().args(["view", "escalations"]).output().unwrap();
    assert!(String::from_utf8_lossy(&out.stdout).contains("(none open)"));

    f.bin()
        .args([
            "escalate",
            "add",
            "plans/expand-retail-doors.md",
            "--by",
            "org/founder",
            "--asks",
            "which distributor terms do we accept?",
            "--attempted",
            "two rounds of negotiation",
        ])
        .assert()
        .success();
    let out = f.bin().args(["view", "escalations"]).output().unwrap();
    let text = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(text.contains("plans/expand-retail-doors.md"), "{text}");
    assert!(text.contains("which distributor terms do we accept?"));

    // The record is well-formed by construction: lint item 24 stays quiet.
    let report = f.lint_json(&["--items", "24"]);
    assert_eq!(report["summary"]["violations"], 0, "{report:#}");
}

#[test]
fn plan_lifecycle_end_to_end() {
    let f = Fixture::healthy();
    let plan = "plans/winter-line.md";
    f.write(
        plan,
        "---\nprovenance: authored\nowner: org/founder\nstatus: draft\ntype: initiative\nsubdomains: [problem/recipe-development.md]\ncontexts: [solution/kit-kitchen]\nmetrics: [metrics/definitions.md#kits-shipped-weekly]\n---\n# Winter line\n\n## Objective\n\nA three-recipe winter line in doors by the spring restock.\n",
    );

    f.bin().args(["plan", "release", plan]).assert().success();
    assert!(f.read(plan).contains("status: ready"));

    f.bin().args(["plan", "claim", plan]).assert().success();
    assert!(f.read(plan).contains("status: active"));

    f.bin()
        .args([
            "plan",
            "block",
            plan,
            "--asks",
            "co-packer or in-house for the winter run?",
        ])
        .assert()
        .success();
    let text = f.read(plan);
    assert!(text.contains("status: blocked"));
    assert!(text.contains("## Escalations"));
    assert!(text.contains("status: open"));
    // Blocked-with-record is exactly what item 24 wants.
    assert_eq!(f.lint_json(&["--items", "24"])["summary"]["violations"], 0);

    f.bin()
        .args(["escalate", "resolve", plan])
        .assert()
        .success();
    assert!(f.read(plan).contains("status: resolved"));

    f.bin().args(["plan", "unblock", plan]).assert().success();
    assert!(f.read(plan).contains("status: ready"));

    f.bin().args(["plan", "retire", plan]).assert().success();
    assert!(f.read(plan).contains("status: retired"));

    // Illegal transition refuses: a retired plan is not claimable.
    f.bin().args(["plan", "claim", plan]).assert().code(2);
}

#[test]
fn release_gates_on_the_mechanical_readiness_share() {
    let f = Fixture::healthy();
    let plan = "plans/vague.md";
    f.write(
        plan,
        "---\nprovenance: authored\nowner: org/founder\nstatus: draft\ntype: initiative\nsubdomains: [problem/missing.md]\n---\n# Vague\n\nWe should TBD the channel strategy.\n",
    );
    f.bin().args(["plan", "release", plan]).assert().code(2);
    assert!(
        f.read(plan).contains("status: draft"),
        "failed release leaves the plan draft"
    );
    f.bin()
        .args(["plan", "release", plan, "--force"])
        .assert()
        .success();
    assert!(f.read(plan).contains("status: ready"));
}

#[test]
fn claim_refuses_a_held_plan() {
    let f = Fixture::healthy();
    f.write(
        "plans/waits.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: ready\ntype: initiative\nawaits: [plans/seasonal-recipe-line.md]\n---\n# Waits\n",
    );
    f.bin()
        .args(["plan", "claim", "plans/waits.md"])
        .assert()
        .code(2);
    assert!(f.read("plans/waits.md").contains("status: ready"));
}

#[test]
fn scaffold_produces_a_pinned_owned_instance() {
    let f = Fixture::healthy(); // only used for a hermetic bin() env
    let target = tempfile::TempDir::new().unwrap();
    let dest = target.path().join("newdomain");
    f.bin()
        .args(["scaffold", dest.to_str().unwrap(), "--owner", "founder"])
        .assert()
        .success();
    let conv = std::fs::read_to_string(dest.join("conventions.md")).unwrap();
    assert!(
        conv.contains("owner: org/founder"),
        "owner placeholder substituted"
    );
    assert!(!conv.contains("<owner>"));
    let adopt = std::fs::read_to_string(dest.join("decisions/0000-adopt-trellis.md")).unwrap();
    assert!(
        adopt.contains("date: 2026-08-03"),
        "adopt decision dated today:\n{adopt}"
    );
    // Derived, so a spec bump re-pins the template rather than this test.
    let pin = format!("specification v{}", trellis::spec_version());
    assert!(
        adopt.contains(&pin),
        "scaffolded instance pins {pin}:\n{adopt}"
    );
    assert!(
        !dest.join("README.md").exists(),
        "template README is not part of an instance"
    );
    // The scaffold is itself a Trellis root.
    f.bin()
        .args(["-C", dest.to_str().unwrap(), "root"])
        .assert()
        .success();
}

#[test]
fn tree_censuses_the_artifacts_and_names_what_it_skipped() {
    let f = Fixture::healthy();
    f.write(".gitignore", "node_modules/\n");
    f.write(
        "solution/kit-kitchen/storefront/node_modules/left-pad/README.md",
        "# left-pad\n",
    );

    let out = f.bin().arg("tree").output().unwrap();
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(
        text.contains("├── ") && text.contains("└── "),
        "the text form draws a tree:\n{text}"
    );
    assert!(
        text.contains("conventions.md") && text.contains("conventions"),
        "artifacts carry their kind:\n{text}"
    );
    assert!(
        !text.contains("left-pad"),
        "the census shows artifacts, not dependencies:\n{text}"
    );
    assert!(
        text.contains("1 git-ignored path(s)"),
        "and says what it skipped:\n{text}"
    );

    // JSON is the same census, machine-shaped.
    let out = f.bin().args(["--format", "json", "tree"]).output().unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let paths: Vec<&str> = report["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"conventions.md"), "{paths:?}");
    assert!(!paths.iter().any(|p| p.contains("left-pad")), "{paths:?}");
    assert!(
        paths.windows(2).all(|w| w[0] <= w[1]),
        "sorted by path: {paths:?}"
    );
    assert_eq!(report["scope"]["git_ignored"].as_array().unwrap().len(), 1);
}

#[test]
fn tree_narrows_by_kind_and_by_path() {
    let f = Fixture::healthy();

    let json = |args: &[&str]| -> Vec<String> {
        let mut cmd = f.bin();
        cmd.args(["--format", "json", "tree"]).args(args);
        let out = cmd.output().unwrap();
        let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        report["artifacts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| a["path"].as_str().unwrap().to_string())
            .collect()
    };

    let mandates = json(&["--kind", "mandate"]);
    assert!(!mandates.is_empty());
    assert!(
        mandates.iter().all(|p| p.ends_with("/mandate.md")),
        "{mandates:?}"
    );

    let under_org = json(&["org"]);
    assert!(!under_org.is_empty());
    assert!(
        under_org.iter().all(|p| p.starts_with("org/")),
        "{under_org:?}"
    );

    // The positional matches on segments, not on text.
    assert!(
        json(&["or"]).is_empty(),
        "a partial segment matches nothing"
    );

    // --flat is the pipeable form: one path per line, nothing else.
    let out = f.bin().args(["tree", "--flat", "org"]).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        text.lines().collect::<Vec<_>>(),
        under_org.iter().map(String::as_str).collect::<Vec<_>>(),
        "--flat prints exactly the paths"
    );
}
