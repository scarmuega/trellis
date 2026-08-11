mod common;

use common::Fixture;

fn gate(f: &Fixture, payload: serde_json::Value) -> (String, bool) {
    let out = f
        .bin()
        .arg("gate")
        .write_stdin(payload.to_string())
        .output()
        .unwrap();
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.success(),
    )
}

fn write_payload(f: &Fixture, rel: &str, content: &str) -> serde_json::Value {
    serde_json::json!({
        "tool_name": "Write",
        "tool_input": { "file_path": rel, "content": content },
        "cwd": f.root().to_string_lossy(),
    })
}

fn edit_payload(f: &Fixture, rel: &str) -> serde_json::Value {
    serde_json::json!({
        "tool_name": "Edit",
        "tool_input": { "file_path": rel, "old_string": "a", "new_string": "b" },
        "cwd": f.root().to_string_lossy(),
    })
}

#[test]
fn denies_committed_accepted_decision() {
    let f = Fixture::healthy();
    let (out, ok) = gate(&f, edit_payload(&f, "decisions/0000-adopt-trellis.md"));
    assert!(ok, "gate always exits 0");
    assert!(
        out.contains("\"permissionDecision\":\"deny\""),
        "expected deny, got: {out}"
    );
    assert!(out.contains("append-only"));
}

#[test]
fn allows_uncommitted_accepted_draft() {
    let f = Fixture::healthy();
    f.write(
        "decisions/0001-fresh-call.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: accepted\ndate: 2026-08-03\n---\n# 0001 — Fresh\n",
    );
    let (out, ok) = gate(&f, edit_payload(&f, "decisions/0001-fresh-call.md"));
    assert!(ok);
    assert!(
        out.is_empty(),
        "an uncommitted draft stays editable, got: {out}"
    );
}

#[test]
fn generated_requires_acting_role_marker() {
    let f = Fixture::healthy();
    let rel = "metrics/actuals/latest.md";
    let (out, _) = gate(&f, edit_payload(&f, rel));
    assert!(
        out.contains("\"permissionDecision\":\"deny\""),
        "expected deny, got: {out}"
    );
    assert!(out.contains("provenance: generated"));

    std::fs::create_dir_all(f.root().join(".trellis")).unwrap();
    std::fs::write(
        f.root().join(".trellis/acting-role"),
        "org/steward 2026-08-03T00:00:00Z\n",
    )
    .unwrap();
    let (out, _) = gate(&f, edit_payload(&f, rel));
    assert!(out.is_empty(), "marker attributes the write, got: {out}");
}

#[test]
fn warns_on_new_artifact_without_frontmatter() {
    let f = Fixture::healthy();
    let (out, _) = gate(&f, write_payload(&f, "problem/new-idea.md", "# An idea\n"));
    assert!(
        out.contains("systemMessage"),
        "expected warning, got: {out}"
    );
    assert!(out.contains("provenance"));

    let (out, _) = gate(
        &f,
        write_payload(
            &f,
            "problem/typed-idea.md",
            "---\nprovenance: authored\nowner: org/founder\n---\n# ok\n",
        ),
    );
    assert!(
        out.is_empty(),
        "frontmattered new artifact passes silently, got: {out}"
    );
}

#[test]
fn inert_outside_a_root_and_on_garbage() {
    let f = Fixture::healthy();
    let outside = tempfile::TempDir::new().unwrap();
    let payload = serde_json::json!({
        "tool_name": "Write",
        "tool_input": { "file_path": "note.md", "content": "# hi" },
        "cwd": outside.path().to_string_lossy(),
    });
    let (out, ok) = gate(&f, payload);
    assert!(
        ok && out.is_empty(),
        "inert outside a Trellis root, got: {out}"
    );

    let out = f
        .bin()
        .arg("gate")
        .write_stdin("this is not json")
        .output()
        .unwrap();
    assert!(out.status.success(), "garbage stdin fails open");
    assert!(out.stdout.is_empty());
}

#[test]
fn fails_open_on_panic() {
    let f = Fixture::healthy();
    let out = f
        .bin()
        .arg("gate")
        .env("TRELLIS_GATE_TEST_PANIC", "1")
        .write_stdin(edit_payload(&f, "decisions/0000-adopt-trellis.md").to_string())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "a broken gate must never brick a session"
    );
    assert!(out.stdout.is_empty());
}

#[test]
fn the_successor_carries_the_edge_the_frozen_target_stays_frozen() {
    let f = Fixture::healthy();
    // Writing a NEW decision that supersedes a committed accepted one is the
    // sanctioned move — it touches no frozen file.
    let (out, ok) = gate(
        &f,
        write_payload(
            &f,
            "decisions/0001-re-adopt.md",
            "---\nprovenance: authored\nowner: org/founder\nstatus: accepted\ndate: 2026-08-03\nsupersedes: [decisions/0000-adopt-trellis.md]\n---\n# 0001 — Re-adopt\n",
        ),
    );
    assert!(ok);
    assert!(
        out.is_empty(),
        "the successor is a new file and stays editable, got: {out}"
    );

    // While the superseded target remains as frozen as ever.
    let (out, _) = gate(&f, edit_payload(&f, "decisions/0000-adopt-trellis.md"));
    assert!(
        out.contains("\"permissionDecision\":\"deny\""),
        "supersession never unfreezes the target, got: {out}"
    );
}
