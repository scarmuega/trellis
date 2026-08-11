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

/// A plan whose `awaits:` names `targets`. The graph reads frontmatter alone,
/// so the body can be a heading.
fn sequenced_plan(f: &Fixture, name: &str, status: &str, targets: &[&str]) {
    let awaits = if targets.is_empty() {
        String::new()
    } else {
        format!("awaits: [{}]\n", targets.join(", "))
    };
    f.write(
        &format!("plans/{name}.md"),
        &format!(
            "---\nprovenance: authored\nowner: org/founder\nstatus: {status}\ntype: initiative\n{awaits}---\n# {name}\n"
        ),
    );
}

#[test]
fn plan_graph_draws_every_live_plan_sequenced_or_not() {
    let f = Fixture::healthy();
    // The healthy fixture sequences nothing, but its two plans are still the
    // board: nodes under no ordering constraint, not an empty figure.
    let out = f.bin().args(["view", "plan-graph"]).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("```mermaid\nflowchart LR\n"), "{text}");
    assert!(
        text.contains(r#"p0["expand-retail-doors<br/>active"]"#),
        "an unsequenced plan is still drawn:\n{text}"
    );
    assert!(text.contains("- drawn: 2 nodes, 0 edges"), "{text}");

    sequenced_plan(&f, "foundation", "retired", &[]);
    sequenced_plan(&f, "pilot", "active", &["plans/foundation.md"]);
    sequenced_plan(&f, "rollout", "ready", &["plans/pilot.md"]);

    let a = f.bin().args(["view", "plan-graph"]).output().unwrap();
    let b = f.bin().args(["view", "plan-graph"]).output().unwrap();
    assert!(a.status.success());
    assert_eq!(
        a.stdout, b.stdout,
        "same tree + HEAD + day ⇒ byte-identical"
    );

    let text = String::from_utf8(a.stdout).unwrap();
    assert!(
        text.starts_with("---\nprovenance: generated\n"),
        "views stamp provenance:\n{text}"
    );
    assert!(text.contains("generated-by: trellis view plan-graph"));

    // A retired plan is off the board: neither the node nor the edge onto it
    // survives, the second being the same statement as its hold having cleared.
    assert!(
        !text.contains("foundation"),
        "retired plans stay out of the figure:\n{text}"
    );
    assert!(!text.contains("classDef retired"), "{text}");
    assert!(text.contains("- retired: 1 plan, left out"), "{text}");

    // Ids are positional over the sorted live node set: expand-retail-doors,
    // pilot, rollout, seasonal-recipe-line. Arrows run target → dependent.
    assert!(text.contains(r#"p1["pilot<br/>active"]"#), "{text}");
    assert!(text.contains(r#"p2["rollout<br/>ready"]"#), "{text}");
    assert!(text.contains("p1 --> p2"), "live hold:\n{text}");
    assert!(text.contains("- drawn: 4 nodes, 1 edge"), "{text}");
    assert!(text.contains("- held: 1 ready plan,"), "{text}");
    assert!(
        text.contains("- unsequenced: 2 plans,"),
        "the fixture's own two plans are drawn, on no edge:\n{text}"
    );
}

#[test]
fn plan_graph_shows_cycles_and_unresolved_targets() {
    let f = Fixture::healthy();
    sequenced_plan(&f, "loop-a", "ready", &["plans/loop-b.md"]);
    sequenced_plan(&f, "loop-b", "ready", &["plans/loop-a.md"]);
    sequenced_plan(&f, "rollout", "ready", &["plans/ghost.md"]);

    let out = f.bin().args(["view", "plan-graph"]).output().unwrap();
    let text = String::from_utf8(out.stdout).unwrap();

    // A target naming no plan is a hole in the picture, not a dropped edge —
    // lint item 4 owns the complaint, the graph shows what it complains about.
    assert!(text.contains(r#"p1["ghost<br/>missing"]"#), "{text}");
    assert!(text.contains("classDef missing"), "{text}");
    assert!(
        text.contains("p1 --> p4"),
        "ghost still holds rollout:\n{text}"
    );
    assert!(text.contains("- unresolved: 1 `awaits:` target,"), "{text}");

    // Cycle members keep their status class and gain `cycle`, whose classDef
    // is emitted last so its stroke wins.
    assert!(text.contains("class p2,p3 cycle"), "{text}");
    assert!(
        text.find("classDef ready").unwrap() < text.find("classDef cycle").unwrap(),
        "cycle styling must come last to win:\n{text}"
    );
    assert!(text.contains("- cycles: 1 cycle"), "{text}");

    // The same tree is an item 22 violation — the view renders what lint
    // reports rather than diverging from it.
    let report = f.lint_json(&["--items", "22"]);
    assert!(finding_items(&report).contains(&22), "{report:#}");
}

#[test]
fn written_plan_graph_satisfies_item_2_until_hand_edited() {
    let f = Fixture::healthy();
    sequenced_plan(&f, "foundation", "retired", &[]);
    sequenced_plan(&f, "pilot", "active", &["plans/foundation.md"]);
    f.bin()
        .args(["view", "plan-graph", "--write"])
        .assert()
        .success();
    f.commit_at("2026-08-03", "steward: refresh plan graph");

    let report = f.lint_json(&["--items", "2"]);
    assert_eq!(
        report["summary"]["violations"], 0,
        "fresh graph diffs clean: {report:#}"
    );
    let checked = &report["judgment"][0]["checked_mechanically"];
    assert_eq!(checked[0], "metrics/actuals/plan-graph.md");
    f.bin()
        .args(["view", "plan-graph", "--check"])
        .assert()
        .success();

    let graph = f.read("metrics/actuals/plan-graph.md");
    f.write(
        "metrics/actuals/plan-graph.md",
        &format!("{graph}\nAn edge somebody drew by hand.\n"),
    );
    let report = f.lint_json(&["--items", "2"]);
    assert!(finding_items(&report).contains(&2), "{report:#}");
    f.bin()
        .args(["view", "plan-graph", "--check"])
        .assert()
        .code(1);
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

#[test]
fn decision_register_reads_the_supersession_edges() {
    let f = Fixture::healthy();
    f.write(
        "decisions/0001-first-call.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: accepted\ndate: 2026-07-01\n---\n# 0001 — First call\n",
    );
    f.write(
        "decisions/0002-second-thoughts.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: accepted\ndate: 2026-07-20\nsupersedes: [decisions/0001-first-call.md]\n---\n# 0002 — Second thoughts\n\n## Consequences\n\nStanding guidance: none.\n",
    );
    f.write(
        "decisions/0003-still-drafting.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: proposed\ndate: 2026-08-01\n---\n# 0003 — Still drafting\n",
    );

    let a = f.bin().args(["view", "decisions"]).output().unwrap();
    let b = f.bin().args(["view", "decisions"]).output().unwrap();
    assert!(a.status.success());
    assert_eq!(a.stdout, b.stdout, "same tree + day ⇒ byte-identical");
    let text = String::from_utf8(a.stdout).unwrap();
    assert!(text.starts_with("---\nprovenance: generated\n"));
    assert!(text.contains("generated-by: trellis view decisions"));
    assert!(
        text.contains("| decisions/0001-first-call.md | decisions/0002-second-thoughts.md |"),
        "superseded table names the successor:\n{text}"
    );
    assert!(
        text.contains("| decisions/0002-second-thoughts.md | inert (standing guidance: none) |"),
        "live table says how each decision is reached:\n{text}"
    );
    assert!(
        !text.contains("| decisions/0003-still-drafting.md |"),
        "a draft is in neither table:\n{text}"
    );
    assert!(text.contains("- not yet accepted: 1"), "{text}");
    // The skeleton's 0000 stays live, cited from conventions.md.
    assert!(
        text.contains("| decisions/0000-adopt-trellis.md | cited by conventions.md |"),
        "{text}"
    );
}

#[test]
fn written_decision_register_satisfies_item_2() {
    let f = Fixture::healthy();
    f.bin()
        .args(["view", "decisions", "--write"])
        .assert()
        .success();
    assert!(f
        .read("metrics/actuals/decision-register.md")
        .contains("# Decision register"));
    f.commit_at("2026-08-03", "steward: refresh decision register");

    let report = f.lint_json(&["--items", "2"]);
    assert_eq!(
        report["summary"]["violations"], 0,
        "fresh register diffs clean: {report:#}"
    );

    let register = f.read("metrics/actuals/decision-register.md");
    f.write(
        "metrics/actuals/decision-register.md",
        &format!("{register}\nA hopeful hand-written line.\n"),
    );
    let report = f.lint_json(&["--items", "2"]);
    assert!(finding_items(&report).contains(&2), "{report:#}");
}
