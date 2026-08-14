//! The terminal tier (`archive/`). The load-bearing property is that moving
//! an artifact into it changes no reading: kind is classified through the
//! prefix, and `gitio` follows the rename, so census, flow, and cycle time
//! are the same before and after. A test that only checked the move happened
//! would miss exactly the failure this design exists to avoid.

mod common;

use common::Fixture;

const PLAN: &str = "---\nprovenance: authored\nowner: org/founder\nstatus: {status}\ntype: initiative\nsubdomains: [problem/outdoor-retail-channel.md]\n---\n# Winter line\n";

fn plan_at(status: &str) -> String {
    PLAN.replace("{status}", status)
}

/// A plan authored well before the window, retired inside it.
fn fixture_with_retired_plan() -> Fixture {
    let f = Fixture::healthy();
    f.write("plans/winter-line.md", &plan_at("draft"));
    f.commit_at("2026-07-20", "author winter line");
    f.write("plans/winter-line.md", &plan_at("retired"));
    f.commit_at("2026-08-01", "retire winter line");
    f
}

fn board(f: &Fixture) -> String {
    let out = f.bin().args(["view", "board"]).output().unwrap();
    assert!(
        out.status.success(),
        "view board failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// The value cell of a `| label | value |` row.
fn row(text: &str, label: &str) -> String {
    text.lines()
        .find(|l| l.starts_with(&format!("| {label} |")))
        .unwrap_or_else(|| panic!("no `{label}` row in:\n{text}"))
        .to_string()
}

fn line_with(text: &str, needle: &str) -> String {
    text.lines()
        .find(|l| l.contains(needle))
        .unwrap_or_else(|| panic!("no line containing `{needle}` in:\n{text}"))
        .to_string()
}

fn archive(f: &Fixture, from: &str, to: &str) {
    std::fs::create_dir_all(f.root().join(to).parent().unwrap()).unwrap();
    f.git(&["mv", from, to]);
    f.commit_at("2026-08-02", "archive winter line");
}

#[test]
fn board_readings_survive_archiving() {
    let f = fixture_with_retired_plan();
    let before = board(&f);

    archive(&f, "plans/winter-line.md", "archive/plans/winter-line.md");
    let after = board(&f);

    // The census still sees a retired plan — classification reads through
    // the tier.
    assert_eq!(row(&before, "retired"), row(&after, "retired"));
    // The closure is still in the window — the rename did not truncate the
    // status timeline. This is the reading that dies without --follow.
    assert_eq!(row(&before, "closed"), row(&after, "closed"));
    assert_eq!(
        line_with(&before, "closures in window:"),
        line_with(&after, "closures in window:")
    );
    assert_eq!(
        line_with(&before, "cycle time"),
        line_with(&after, "cycle time")
    );
}

#[test]
fn closure_is_actually_counted_before_the_move() {
    // Guards the test above from passing vacuously: if `closed` were 0 both
    // times, the invariance assertion would prove nothing.
    let f = fixture_with_retired_plan();
    assert_eq!(row(&board(&f), "closed"), "| closed | 1 |");
}

/// The whole point of keeping the live path canonical: an `awaits:` edge is
/// written once and keeps meaning the same plan after that plan is archived.
/// Retirement is what clears a hold, and archiving happens *after*
/// retirement — so if the move broke the edge, every dependent would jam
/// exactly when it was supposed to be released.
#[test]
fn an_awaits_edge_survives_its_target_being_archived() {
    let f = fixture_with_retired_plan();
    f.write(
        "plans/spring-line.md",
        &plan_at("ready").replace("# Winter line", "# Spring line"),
    );
    f.write(
        "plans/spring-line.md",
        &f.read("plans/spring-line.md").replace(
            "type: initiative",
            "type: initiative\nawaits: [plans/winter-line.md]",
        ),
    );
    f.commit_at("2026-08-01", "queue spring behind winter");

    let held_before = f
        .bin()
        .args(["query", "held", "plans/spring-line.md"])
        .output()
        .unwrap();
    let before = String::from_utf8_lossy(&held_before.stdout)
        .trim()
        .to_string();
    assert_eq!(before, "not held", "winter is retired, so spring is free");

    archive(&f, "plans/winter-line.md", "archive/plans/winter-line.md");

    let held_after = f
        .bin()
        .args(["query", "held", "plans/spring-line.md"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&held_after.stdout).trim(),
        "not held",
        "archiving the target must not re-hold the dependent"
    );

    // And the edge still resolves — no dangling-ref finding against item 4.
    let report = f.lint_json(&[]);
    let dangling: Vec<String> = report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["message"].as_str().unwrap_or("").contains("awaits"))
        .map(|f| f["message"].as_str().unwrap().to_string())
        .collect();
    assert!(dangling.is_empty(), "awaits edge broke: {dangling:?}");
}

fn items_for(f: &Fixture, path: &str) -> Vec<u8> {
    f.lint_json(&[])["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["path"].as_str() == Some(path))
        .map(|v| v["item"].as_u64().unwrap() as u8)
        .collect()
}

#[test]
fn a_well_formed_archive_is_clean() {
    let f = fixture_with_retired_plan();
    archive(&f, "plans/winter-line.md", "archive/plans/winter-line.md");
    assert!(
        !items_for(&f, "archive/plans/winter-line.md").contains(&25),
        "a retired plan in the mirror is exactly what the tier is for"
    );
}

#[test]
fn the_tier_refuses_what_does_not_belong() {
    let f = Fixture::healthy();

    // Not a mirror of the live hierarchy — nothing would look for it here.
    f.write(
        "archive/notes.md",
        "---\nprovenance: authored\nowner: org/founder\n---\n# Notes\n",
    );
    // Append-only shared memory is superseded, never filed away (rule 6).
    f.write(
        "archive/decisions/0001-old.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: accepted\n---\n# Old\n",
    );
    // Admission is terminal-only.
    f.write("archive/plans/still-going.md", &plan_at("active"));
    f.commit_at("2026-08-01", "malformed archive");

    assert!(items_for(&f, "archive/notes.md").contains(&25));
    assert!(items_for(&f, "archive/decisions/0001-old.md").contains(&25));
    assert!(items_for(&f, "archive/plans/still-going.md").contains(&25));
}

#[test]
fn a_ref_may_not_name_the_tier() {
    let f = fixture_with_retired_plan();
    archive(&f, "plans/winter-line.md", "archive/plans/winter-line.md");
    f.write(
        "plans/spring-line.md",
        &plan_at("ready").replace(
            "type: initiative",
            "type: initiative\nawaits: [archive/plans/winter-line.md]",
        ),
    );
    f.commit_at("2026-08-02", "ref the tier");

    assert!(
        items_for(&f, "plans/spring-line.md").contains(&25),
        "the tier is a location, not an address — the live path is the ref"
    );
}

#[test]
fn the_verb_files_a_terminal_artifact_and_refuses_a_live_one() {
    let f = fixture_with_retired_plan();
    f.write("plans/spring-line.md", &plan_at("active"));
    f.commit_at("2026-08-01", "spring is in flight");

    let live = f
        .bin()
        .args(["archive", "plans/spring-line.md"])
        .output()
        .unwrap();
    assert!(!live.status.success(), "an active plan must not be filed");
    assert!(
        String::from_utf8_lossy(&live.stderr).contains("terminal artifacts only"),
        "{}",
        String::from_utf8_lossy(&live.stderr)
    );

    f.bin()
        .args(["archive", "plans/winter-line.md"])
        .assert()
        .success();
    assert!(
        f.root().join("archive/plans/winter-line.md").exists(),
        "the retired plan should have moved into the tier"
    );
    assert!(!f.root().join("plans/winter-line.md").exists());
}

#[test]
fn the_sweep_needs_a_declared_horizon_and_honours_it() {
    let f = fixture_with_retired_plan();

    // Nothing declared: the kernel invents no retention policy.
    let bare = f.bin().args(["archive", "--sweep"]).output().unwrap();
    assert!(!bare.status.success());
    assert!(
        String::from_utf8_lossy(&bare.stderr).contains("no retention horizon declared"),
        "{}",
        String::from_utf8_lossy(&bare.stderr)
    );

    // Retired on 2026-08-01, anchor is 2026-08-03 — two days cold.
    let config = f.read("trellis.toml");
    f.write(
        "trellis.toml",
        &format!("{config}\n[archive]\nafter_days = 30\n"),
    );
    f.commit_at("2026-08-01", "declare a 30-day horizon");
    let warm = f
        .bin()
        .args(["archive", "--sweep", "--dry-run"])
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&warm.stdout).contains("nothing to file"),
        "two days is not thirty: {}",
        String::from_utf8_lossy(&warm.stdout)
    );

    let config = f.read("trellis.toml");
    f.write(
        "trellis.toml",
        &config.replace("after_days = 30", "after_days = 1"),
    );
    f.commit_at("2026-08-01", "shorten the horizon");
    let cold = f
        .bin()
        .args(["archive", "--sweep", "--dry-run"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&cold.stdout);
    assert!(
        text.contains("would file plans/winter-line.md"),
        "a two-day-cold plan is past a one-day horizon: {text}"
    );
    // --dry-run means exactly that.
    assert!(f.root().join("plans/winter-line.md").exists());
}

#[test]
fn archived_plan_keeps_its_kind() {
    let f = fixture_with_retired_plan();
    archive(&f, "plans/winter-line.md", "archive/plans/winter-line.md");

    let out = f
        .bin()
        .args(["tree", "--kind", "plan", "--flat"])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("archive/plans/winter-line.md"),
        "archived plan is not a plan:\n{text}"
    );
}
