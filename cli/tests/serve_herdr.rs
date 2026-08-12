//! The herdr binding, hermetically: the channel adapter announcing
//! escalations, and the opt-in session backend running a plan's session as a
//! pane — against a fake herdr server, the same way every other serve test
//! runs against a fake harness (decision 0043).

#![cfg(unix)]

mod common;

use common::{herdr_agent, FakeHerdr, Fixture, ANCHOR};

// The subdomain keeps ready fixtures past the scan's mechanical readiness
// precheck (decision 0045).
const FM: &str =
    "---\nprovenance: authored\nowner: org/founder\nsubdomains: [problem/outdoor-retail-channel.md]\n";

fn fake(f: &Fixture, statuses: &[&str], agents: Vec<serde_json::Value>) -> FakeHerdr {
    FakeHerdr::start(f.root().join(".trellis/herdr.sock"), statuses, agents)
}

/// Process backend with the claiming fake harness, plus the herdr channel.
fn channel_fixture(f: &Fixture, socket: &std::path::Path) {
    let harness = f.fake_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\"]\n\n\
             [[channels]]\n\
             kind = \"herdr\"\n\
             socket = \"{}\"\n",
            socket.display()
        ),
    );
}

fn herdr_backend_fixture(f: &Fixture, socket: &std::path::Path) {
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\nbackend = \"herdr\"\n\n\
             [harness.herdr]\nsocket = \"{}\"\n",
            socket.display()
        ),
    );
}

const BLOCKED_WITH_RECORD: &str = "---\nprovenance: authored\nowner: org/founder\n\
    status: blocked\ntype: initiative\n---\n# Ship\n\n## Escalations\n\n\
    ```yaml\nraised: 2026-08-04\nby: org/founder\nto: org/founder\nstatus: open\nasks: unblock me\n```\n";

#[test]
fn a_newly_opened_escalation_toasts_once_and_only_once() {
    let f = Fixture::healthy();
    let herdr = fake(&f, &[], Vec::new());
    channel_fixture(&f, &herdr.socket);

    // First pass seeds: the standing backlog is not news.
    f.dispatch_once(ANCHOR, &[]);
    assert_eq!(herdr.params_for("notification.show").len(), 0);

    f.write("plans/ship-it.md", BLOCKED_WITH_RECORD);
    f.dispatch_once("2026-08-04", &[]);
    let shown = herdr.params_for("notification.show");
    assert_eq!(shown.len(), 1, "one record, one toast: {shown:?}");
    let title = shown[0]["title"].as_str().unwrap();
    assert!(title.contains("plans/ship-it.md"), "{title}");
    assert!(title.contains("org/founder"), "{title}");
    assert_eq!(shown[0]["body"], "unblock me");

    // The record is still open on the next pass, and already announced.
    f.dispatch_once("2026-08-05", &[]);
    assert_eq!(herdr.params_for("notification.show").len(), 1, "announced once");
}

#[test]
fn a_configured_channel_with_no_herdr_behind_it_refuses_startup() {
    let f = Fixture::healthy();
    channel_fixture(&f, &f.root().join(".trellis/nobody-home.sock"));
    let out = f
        .bin()
        .args(["dispatch", "run", "--once"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("cannot reach herdr"), "{err}");
}

#[test]
fn the_herdr_backend_runs_a_session_as_a_workspace_pane() {
    let f = Fixture::healthy();
    let herdr = fake(&f, &["idle"], Vec::new());
    herdr_backend_fixture(&f, &herdr.socket);
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );

    let out = f.dispatch_once(ANCHOR, &[]);
    assert!(out.contains("herdr backend: fake"), "{out}");
    assert!(out.contains("started act plans/ship-it.md"), "{out}");

    // The lifecycle, in order: a workspace, identity on its pane, the agent,
    // the prompt, the wait, the scrollback, the cleanup.
    let calls = herdr.calls();
    let position = |m: &str| {
        calls
            .iter()
            .position(|c| c == m)
            .unwrap_or_else(|| panic!("no {m} in {calls:?}"))
    };
    assert!(position("workspace.create") < position("pane.report_metadata"));
    assert!(position("pane.report_metadata") < position("agent.start"));
    assert!(position("agent.start") < position("agent.prompt"));
    assert!(position("agent.prompt") < position("agent.wait"));
    assert!(position("agent.wait") < position("agent.read"));
    assert!(position("agent.read") < position("workspace.close"));

    let start = &herdr.params_for("agent.start")[0];
    assert_eq!(start["kind"], "claude");
    assert_eq!(start["pane_id"], "p1");
    assert_eq!(
        start["name"], "t-plan-plans-ship-it-md",
        "a herdr-legal name: lowercase, no dots, ≤32 chars"
    );
    let args: Vec<&str> = start["args"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| a.as_str().unwrap())
        .collect();
    assert!(args.contains(&"--model") && args.contains(&"opus"), "{args:?}");
    assert!(args.contains(&"--effort") && args.contains(&"high"), "{args:?}");
    assert!(
        !args.contains(&"--max-budget-usd"),
        "no budget flag: interactive claude cannot enforce one ({args:?})"
    );

    let prompt = &herdr.params_for("agent.prompt")[0];
    assert_eq!(prompt["target"], "p1");
    let text = prompt["text"].as_str().unwrap();
    // The rendered default (decision 0050): a discriminating first line, the
    // computed facts, and the act procedure — no slash command to resolve.
    assert!(
        text.starts_with("plans/ship-it.md — dispatched act as founder"),
        "{text}"
    );
    assert!(text.contains("Bind to the domain root"), "{text}");

    let tokens = &herdr.params_for("pane.report_metadata")[0]["tokens"];
    assert_eq!(tokens["trellis_key"], "plan:plans/ship-it.md");
    assert!(tokens["trellis_log"].as_str().unwrap().ends_with(".log"));

    // The run is bookkept like any other, and the log holds the pane's tail.
    let state = f.state();
    assert_eq!(state["runs"]["plan:plans/ship-it.md"]["last_exit"], 0);
    let log = tokens["trellis_log"].as_str().unwrap();
    let text = f.read(&format!(".trellis/runtime/logs/{log}"));
    assert!(text.contains("# act plans/ship-it.md → org/founder"), "{text}");
    assert!(text.contains("fake scrollback tail"), "{text}");
}

/// Observed live: herdr accepts `agent.prompt` while Claude Code is still
/// starting up and the injected text vanishes — the pane settles `idle` over
/// an empty input. The pool must not read that as a finished turn; it puts
/// the prompt back in.
#[test]
fn a_prompt_dropped_at_startup_is_resubmitted() {
    let f = Fixture::healthy();
    let herdr = FakeHerdr::start_dropping(
        f.root().join(".trellis/herdr.sock"),
        &["idle"],
        Vec::new(),
        1,
    );
    herdr_backend_fixture(&f, &herdr.socket);
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );

    let out = f.dispatch_once(ANCHOR, &[]);
    assert!(out.contains("prompt was dropped at startup — resubmitted (1/2)"), "{out}");

    let prompts = herdr.params_for("agent.prompt");
    assert_eq!(prompts.len(), 2, "the drop, then the resubmission: {prompts:?}");
    assert_eq!(prompts[0]["text"], prompts[1]["text"], "the same prompt goes back in");
    assert_eq!(
        f.state()["runs"]["plan:plans/ship-it.md"]["last_exit"], 0,
        "the resubmitted turn is the run"
    );
}

#[test]
fn a_session_that_never_takes_its_prompt_is_written_off_not_declared_done() {
    let f = Fixture::healthy();
    let herdr = FakeHerdr::start_dropping(
        f.root().join(".trellis/herdr.sock"),
        &["idle"],
        Vec::new(),
        usize::MAX,
    );
    herdr_backend_fixture(&f, &herdr.socket);
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );

    let out = f.dispatch_once(ANCHOR, &[]);
    assert!(out.contains("prompt was never taken up — writing the session off"), "{out}");

    assert_eq!(
        herdr.params_for("agent.prompt").len(),
        3,
        "the drop, then the whole resubmission budget"
    );
    assert_ne!(
        f.state()["runs"]["plan:plans/ship-it.md"]["last_exit"], 0,
        "never bookkept as a success"
    );
    assert!(
        !herdr.calls().iter().any(|c| c == "workspace.close"),
        "retain = on-failure keeps the scene for attach"
    );
}

#[test]
fn an_adopted_session_holds_its_slot_and_is_never_spawned_twice() {
    let f = Fixture::healthy();
    // What the daemon's own cwd resolves to — on macOS the tempdir path and
    // its physical path differ, and adoption matches on the physical one.
    let root = f.root().canonicalize().unwrap();
    let adopted = herdr_agent(
        "plan:plans/ship-it.md",
        &root,
        "act plans/ship-it.md → org/founder",
        "working",
    );
    // First observation still working; the drain then settles it.
    let herdr = fake(&f, &["working", "idle"], vec![adopted]);
    herdr_backend_fixture(&f, &herdr.socket);
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );

    let out = f.dispatch_once(ANCHOR, &[]);
    assert!(out.contains("adopted act plans/ship-it.md"), "{out}");
    assert!(out.contains("already in flight"), "{out}");

    let calls = herdr.calls();
    assert!(
        !calls.iter().any(|c| c == "workspace.create" || c == "agent.start"),
        "the adopted session is the session — nothing spawned: {calls:?}"
    );
    assert_eq!(
        f.state()["runs"]["plan:plans/ship-it.md"]["last_exit"], 0,
        "and the drain settled it"
    );
}

#[test]
fn the_herdr_backend_with_no_herdr_behind_it_refuses_startup() {
    let f = Fixture::healthy();
    herdr_backend_fixture(&f, &f.root().join(".trellis/nobody-home.sock"));
    let out = f
        .bin()
        .args(["dispatch", "run", "--once"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let err = String::from_utf8_lossy(&out.stderr);
    assert!(err.contains("backend"), "{err}");
    assert!(err.contains("cannot reach herdr"), "{err}");
}

#[test]
fn a_parked_question_toasts_with_the_command_that_answers_it() {
    use std::io::BufRead;
    use std::time::{Duration, Instant};

    let f = Fixture::healthy();
    let herdr = fake(&f, &[], Vec::new());
    let harness = f.asking_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\", \"--mcp-config\", \"{{mcp}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\", \"--mcp-config\", \"{{mcp}}\"]\n\n\
             [[channels]]\nkind = \"herdr\"\nsocket = \"{}\"\n",
            herdr.socket.display()
        ),
    );
    f.write(
        "plans/split-billing.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Split billing\n"),
    );

    // A live daemon on a short tick: the session parks its question during
    // one tick, the next tick pushes it to the channels.
    let mut child = std::process::Command::new(Fixture::bin_path())
        .args(["dispatch", "run", "--tick-secs", "1"])
        .current_dir(f.root())
        .env("TRELLIS_TODAY", common::ANCHOR)
        .env_remove("CLAUDE_PLUGIN_ROOT")
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    std::thread::spawn(move || std::io::BufReader::new(stdout).lines().for_each(drop));

    let deadline = Instant::now() + Duration::from_secs(30);
    let toast = loop {
        let asks: Vec<serde_json::Value> = herdr
            .params_for("notification.show")
            .into_iter()
            .filter(|p| p["title"].as_str().unwrap_or("").starts_with("session asks:"))
            .collect();
        if let Some(toast) = asks.first() {
            break toast.clone();
        }
        assert!(Instant::now() < deadline, "no ask toast arrived");
        std::thread::sleep(Duration::from_millis(200));
    };
    let _ = std::process::Command::new("kill")
        .arg(child.id().to_string())
        .status();
    let _ = child.wait();

    assert!(
        toast["title"]
            .as_str()
            .unwrap()
            .contains("act plans/split-billing.md → org/founder"),
        "{toast}"
    );
    let body = toast["body"].as_str().unwrap();
    assert!(body.contains("does the table move?"), "{body}");
    assert!(body.contains("trellis inbox answer"), "{body}");
}

#[test]
fn a_dry_run_under_the_herdr_backend_reports_and_touches_nothing() {
    let f = Fixture::healthy();
    let herdr = fake(&f, &[], Vec::new());
    herdr_backend_fixture(&f, &herdr.socket);
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Ship\n"),
    );

    let out = f.dispatch_once(ANCHOR, &["--dry-run"]);
    assert!(out.contains("would run act plans/ship-it.md → org/founder in herdr"), "{out}");
    let calls = herdr.calls();
    assert!(
        !calls.iter().any(|c| c != "ping" && c != "agent.list"),
        "a dry run only looked: {calls:?}"
    );
}
