//! Lockstep with the model (decision 0037).
//!
//! The kernel is a *second encoding* of things the prose already defines: the
//! lint checklist, the readiness checklist, the spec version, the closed
//! enums. A second encoding is only safe if drift cannot merge quietly, so
//! each of these asserts the encodings still agree. A failure here is not a
//! bug in the kernel — it is a model change that has not been translated yet,
//! and the fix is to land both halves in the same commit.

mod common;

use common::repo_path;

fn read(rel: &str) -> String {
    std::fs::read_to_string(repo_path(rel)).unwrap_or_else(|e| panic!("cannot read {rel}: {e}"))
}

/// Top-level numbered items (`^N. `) in a checklist file.
fn checklist_items(rel: &str) -> Vec<u8> {
    let text = read(rel);
    let re = regex::Regex::new(r"(?m)^(\d+)\.\s").unwrap();
    let mut items: Vec<u8> = re
        .captures_iter(&text)
        .filter_map(|c| c[1].parse().ok())
        .collect();
    items.sort_unstable();
    items.dedup();
    assert!(
        !items.is_empty(),
        "{rel} parsed as zero items — the checklist format changed"
    );
    items
}

#[test]
fn lint_rules_cover_exactly_the_checklist() {
    let checklist = checklist_items("checks/conventions-lint.md");
    let implemented = trellis::lint::implemented_items();

    let missing: Vec<u8> = checklist
        .iter()
        .copied()
        .filter(|i| !implemented.contains(i))
        .collect();
    let extra: Vec<u8> = implemented
        .iter()
        .copied()
        .filter(|i| !checklist.contains(i))
        .collect();

    assert!(
        missing.is_empty(),
        "checks/conventions-lint.md declares item(s) {missing:?} that the kernel does not implement — \
         add a rule to lint::RULES (mechanical) or a judgment note (not mechanically checkable), \
         in the same commit as the checklist change (decision 0037)"
    );
    assert!(
        extra.is_empty(),
        "the kernel implements item(s) {extra:?} that checks/conventions-lint.md no longer declares — \
         the checklist is the single source; drop the rule or restore the item"
    );
}

#[test]
fn readiness_reports_exactly_the_readiness_checklist() {
    let checklist = checklist_items("checks/plan-readiness.md");
    // The readiness report walks every item, mechanical or judgment, so its
    // item set must match the prose one-for-one.
    let f = common::Fixture::healthy();
    let out = f
        .bin()
        .args([
            "readiness",
            "plans/seasonal-recipe-line.md",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    let report: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut reported: Vec<u8> = report["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["item"].as_u64().unwrap() as u8)
        .collect();
    reported.sort_unstable();

    assert_eq!(
        reported, checklist,
        "checks/plan-readiness.md and the kernel's readiness walk disagree on the item set — \
         translate the checklist change into readiness.rs in the same commit (decision 0037)"
    );
}

#[test]
fn embedded_spec_version_matches_every_instance_pin() {
    let title = read("spec/model.md");
    let title = title.lines().next().unwrap_or_default().to_string();
    let spec: u32 = title
        .split("(v")
        .nth(1)
        .and_then(|r| r.split(')').next())
        .and_then(|d| d.parse().ok())
        .unwrap_or_else(|| panic!("spec/model.md title carries no (vN): {title:?}"));

    assert_eq!(
        trellis::spec_version(),
        spec,
        "the binary embedded spec v{} but spec/model.md is v{spec} — rebuild (build.rs reads the title)",
        trellis::spec_version()
    );

    // A spec bump must re-pin every shipped instance, or lint item 18 fires in
    // every new root. This has bitten before: the v14 bump left all three pins
    // stale at v13 (see CHANGELOG, decision 0032's entry).
    let pinned = regex::Regex::new(r"(?i)spec(?:ification)?[,]?\s+v(\d+)").unwrap();
    for rel in [
        "template/decisions/0000-adopt-trellis.md",
        "evals/skeleton/decisions/0000-adopt-trellis.md",
        "README.md",
    ] {
        let text = read(rel);
        let pin: u32 = pinned
            .captures(&text)
            .and_then(|c| c[1].parse().ok())
            .unwrap_or_else(|| panic!("{rel} pins no spec version"));
        assert_eq!(
            pin, spec,
            "{rel} pins spec v{pin} but the spec is v{spec} — re-pin it in the bump commit \
             (conventions-lint item 18 fires on every root scaffolded from a stale pin)"
        );
    }
}

#[test]
fn closed_enums_match_the_schema_prose() {
    // The spec's closed vocabularies are duplicated in model.rs. Each must
    // still appear verbatim in the schema block that defines it.
    let conventions = read("template/conventions.md");
    for (field, values) in [
        (
            "status",
            vec!["draft", "ready", "active", "blocked", "retired"],
        ),
        ("complexity", vec!["mechanical", "standard", "deep"]),
        (
            "strategy status",
            vec![
                "raw",
                "defined",
                "validated",
                "implemented",
                "established",
                "discarded",
            ],
        ),
        ("class", vec!["core", "supporting", "generic"]),
        ("provenance", vec!["authored", "generated", "state-ref"]),
    ] {
        for value in values {
            assert!(
                conventions.contains(value),
                "template/conventions.md no longer mentions `{value}` for {field} — \
                 the kernel's model.rs still parses it; translate the schema change (decision 0037)"
            );
        }
    }
}
