//! Errand requests: an operator asks a plan's owner to carry out one
//! instruction over it, and the dispatch loop — still the only spawner —
//! fires the session (decisions 0048, 0060). There is no errand name and no
//! errand table: the ask is the instruction, and the size it runs at is
//! chosen at the call. Honored locally on the dispatcher's socket, relayed
//! by serve, every other non-GET refused.

mod common;

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use common::{Fixture, ANCHOR};

/// A live daemon — the dispatcher (prints `channel on …`) or serve (prints
/// `listening on …`), parameterized because the relay tests need both.
struct Daemon {
    child: Child,
    port: u16,
}

impl Daemon {
    fn start(f: &Fixture, args: &[&str], listen_prefix: &str) -> Daemon {
        let mut child = Command::new(Fixture::bin_path())
            .args(args)
            .current_dir(f.root())
            .env("TRELLIS_TODAY", ANCHOR)
            .env_remove("CLAUDE_PLUGIN_ROOT")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();

        let stdout = child.stdout.take().unwrap();
        let mut lines = BufReader::new(stdout).lines();
        let port = loop {
            match lines.next() {
                Some(Ok(line)) => {
                    if let Some(addr) = line.strip_prefix(listen_prefix) {
                        break addr.rsplit(':').next().unwrap().parse().unwrap();
                    }
                }
                _ => {
                    let mut err = String::new();
                    if let Some(mut e) = child.stderr.take() {
                        let _ = e.read_to_string(&mut err);
                    }
                    panic!("the daemon exited before it listened: {err}");
                }
            }
        };
        std::thread::spawn(move || lines.for_each(drop));
        Daemon { child, port }
    }

    fn dispatch(f: &Fixture) -> Daemon {
        // An hour between ticks: anything that starts, started on the wake.
        Daemon::start(f, &["dispatch", "run", "--tick-secs", "3600"], "channel on ")
    }

    fn serve(f: &Fixture) -> Daemon {
        Daemon::start(f, &["serve", "--port", "0"], "listening on ")
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        // `SIGTERM`, not `kill()`: the daemon's own stop path removes its
        // pid and addr files, which the relay tests rely on.
        let _ = Command::new("kill")
            .arg(self.child.id().to_string())
            .status();
        let _ = self.child.wait();
    }
}

/// A root with a draft plan to work on. Drafts are never scanned, so the
/// only sessions these tests see are the ones they asked for.
fn errand_fixture() -> Fixture {
    let f = Fixture::healthy();
    let harness = f.fake_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{prompt}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\"]\n"
        ),
    );
    f.write(
        "plans/reshape-me.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: draft\ntype: initiative\n---\n# Reshape me\n",
    );
    f
}

fn errand(port: u16, slug: &str, instruction: &str) -> (u16, serde_json::Value) {
    sized_errand(port, slug, instruction, serde_json::json!({}))
}

/// The same ask with `model`/`effort` named at the call (decision 0060).
fn sized_errand(
    port: u16,
    slug: &str,
    instruction: &str,
    extra: serde_json::Value,
) -> (u16, serde_json::Value) {
    let mut body = serde_json::json!({ "instruction": instruction });
    for (k, v) in extra.as_object().cloned().unwrap_or_default() {
        body[k] = v;
    }
    let (code, text) = post(port, &format!("/api/plans/{slug}/errand"), &body.to_string());
    let value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    (code, value)
}

fn until<T>(what: &str, mut f: impl FnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        if let Some(value) = f() {
            return value;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    panic!("timed out waiting for {what}");
}

#[test]
fn an_errand_spawns_on_the_wake_and_carries_the_instruction() {
    let f = errand_fixture();
    let daemon = Daemon::dispatch(&f);

    let (code, outcome) = errand(daemon.port, "reshape-me", "split the scope");
    assert_eq!(code, 200, "{outcome}");
    assert_eq!(outcome["outcome"], "requested");
    assert_eq!(outcome["owner"], "org/founder");
    assert_eq!(outcome["plan"], "plans/reshape-me.md");
    // Absent a choice at the call, the plan's complexity tier sizes it — the
    // fixture declares none, so standard (decision 0060).
    assert_eq!(outcome["complexity"], "standard");
    assert_eq!(outcome["model"], "opus");
    assert_eq!(outcome["effort"], "high");

    // The tick is an hour away, so only the wake can have started this — and
    // the prompt the harness got is the rendered framework procedure
    // (decision 0050): unique first line, instruction verbatim, the act body
    // it delegates to.
    let argv = until("the errand session to start", || {
        f.invocations().into_iter().next()
    });
    assert_eq!(
        argv[0],
        "errand on plans/reshape-me.md as founder (trellis runtime)."
    );
    let prompt = argv.join("\n");
    assert!(prompt.contains("split the scope"), "{prompt}");
    assert!(prompt.contains("Bind to the domain root"), "{prompt}");
    // The framing carries the execution context an ask may need, unlike the
    // refinement template it replaced.
    assert!(prompt.contains("## Mandate"), "{prompt}");
    // The fixture's founder holder declares no `kind:` — undeclared renders
    // an empty `{delegation}`. (The act body always mentions the disposition;
    // the *note* is what an undeclared holder must not get.)
    assert!(
        !prompt.contains("This role's holder is a declared human"),
        "{prompt}"
    );
}

/// The one thing that varies per ask (decision 0060): the operator's choice
/// beats the plan's tier, and reaches the harness argv.
#[test]
fn model_and_effort_are_chosen_at_the_call() {
    let f = errand_fixture();
    // A harness command that echoes the two values back to us.
    let harness = f.fake_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{prompt}}\", \"{{model}}\", \"{{effort}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\"]\n"
        ),
    );
    let daemon = Daemon::dispatch(&f);

    let (code, outcome) = sized_errand(
        daemon.port,
        "reshape-me",
        "audit the done criteria",
        serde_json::json!({ "model": "fable", "effort": "xhigh" }),
    );
    assert_eq!(code, 200, "{outcome}");
    // The response states what will actually run, not the tier's default.
    assert_eq!(outcome["model"], "fable");
    assert_eq!(outcome["effort"], "xhigh");
    assert_eq!(outcome["complexity"], "standard", "the tier is still reported");

    // The prompt is one argument carrying newlines, and the fake harness
    // records a line per line — so the two trailing arguments are the tail.
    let argv = until("the sized session to start", || {
        f.invocations().into_iter().next()
    });
    assert_eq!(&argv[argv.len() - 2..], ["fable", "xhigh"], "{argv:?}");
}

/// The gate that used to stop here (decision 0057): a declared-human holder
/// is a handoff for the runtime's own triggers, but an errand is the
/// holder's own ask — it spawns, and the prompt names the disposition so the
/// session proceeds under the mandate at their direction instead of stopping
/// at act's never-impersonate branch.
#[test]
fn a_human_held_owner_is_delegation_not_a_refusal() {
    let f = errand_fixture();
    f.write(
        "org/founder/holder/ref.md",
        "---\nprovenance: authored\nowner: org/founder\nref: the founder, a human\nkind: human\n---\n# Holder\n",
    );
    let daemon = Daemon::dispatch(&f);

    let (code, outcome) = errand(daemon.port, "reshape-me", "tighten the scope");
    assert_eq!(code, 200, "a holder's own ask is no refusal: {outcome}");
    assert_eq!(outcome["outcome"], "requested");
    assert_eq!(outcome["delegated"], true, "the route says so too");

    let argv = until("the delegated session to start", || {
        f.invocations().into_iter().next()
    });
    let prompt = argv.join("\n");
    assert!(
        prompt.contains("delegated execution (act's holder branch, decision 0057)"),
        "the prompt names the disposition:\n{prompt}"
    );
    assert!(
        prompt.contains("`authority: approve` stays theirs"),
        "approvals stay personal:\n{prompt}"
    );
}

#[test]
fn the_errand_prompt_carries_the_execution_context() {
    // `refine`'s template withheld the skill index and the role context
    // because refinement was defined as never executing (0055, 0058). An
    // errand has no such definition — the instruction decides what it is —
    // so the framing hands over what any ask might need (decision 0060).
    let f = errand_fixture();
    f.write(
        "org/founder/holder/skills/door-outreach/README.md",
        "# Door outreach\n",
    );
    let daemon = Daemon::dispatch(&f);

    let (code, outcome) = errand(daemon.port, "reshape-me", "tighten the scope");
    assert_eq!(code, 200, "{outcome}");

    let argv = until("the errand session to start", || {
        f.invocations().into_iter().next()
    });
    let prompt = argv.join("\n");
    assert!(prompt.contains("## Skills"), "{prompt}");
    assert!(prompt.contains("door-outreach"), "{prompt}");
    assert!(prompt.contains("## Mandate"), "{prompt}");
}

#[test]
fn refusals_speak_the_tree_vocabulary_and_spawn_nothing() {
    let f = errand_fixture();
    f.write(
        "plans/shipped.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: retired\ntype: initiative\n---\n# Shipped\n",
    );
    f.write(
        "plans/orphan.md",
        "---\nprovenance: authored\nstatus: draft\ntype: initiative\n---\n# Orphan\n",
    );
    let daemon = Daemon::dispatch(&f);

    let (code, v) = errand(daemon.port, "never-heard-of-it", "split it");
    assert_eq!(code, 404, "{v}");
    assert_eq!(v["outcome"], "not-a-plan");

    let (code, v) = errand(daemon.port, "shipped", "split it");
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "terminal");

    let (code, v) = errand(daemon.port, "orphan", "split it");
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "unowned");

    let (code, _) = post(daemon.port, "/api/plans/reshape-me/errand", "{}");
    assert_eq!(code, 400, "an absent instruction is refused, not defaulted");

    std::thread::sleep(Duration::from_millis(600));
    assert!(
        f.invocations().is_empty(),
        "a refusal must never reach the spawner"
    );
}

#[test]
fn a_second_request_while_the_first_runs_is_refused() {
    let f = Fixture::healthy();
    // A harness that outlives both requests, so the first session is still
    // in flight when the second ask arrives.
    let rel = ".trellis/bin/slow";
    let path = f.root().join(rel);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "#!/bin/sh\nsleep 5\nexit 0\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{rel}\", \"{{prompt}}\"]\n\
             ritual_cmd = [\"{rel}\", \"{{ritual}}\"]\n"
        ),
    );
    f.write(
        "plans/reshape-me.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: draft\ntype: initiative\n---\n# Reshape me\n",
    );
    let daemon = Daemon::dispatch(&f);

    let (code, _) = errand(daemon.port, "reshape-me", "split the scope");
    assert_eq!(code, 200);
    until("the session to show in flight", || {
        let (_, status) = get(daemon.port, "/api/status");
        status["in_flight"]
            .as_array()?
            .iter()
            .any(|s| s["key"] == "plan:plans/reshape-me.md")
            .then_some(())
    });

    let (code, v) = errand(daemon.port, "reshape-me", "split it differently");
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "in-flight");
}

#[test]
fn serve_relays_to_the_dispatcher_and_says_so_when_there_is_none() {
    let f = errand_fixture();
    let serve = Daemon::serve(&f);

    // No dispatcher yet: serve refuses to pretend, and names the fix.
    let (code, v) = errand(serve.port, "reshape-me", "split the scope");
    assert_eq!(code, 503, "{v}");
    assert!(
        v["error"].as_str().unwrap().contains("trellis dispatch run"),
        "{v}"
    );

    // With one running, the same POST to *serve* spawns — the board's path.
    let dispatcher = Daemon::dispatch(&f);
    let (code, v) = errand(serve.port, "reshape-me", "split the scope");
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["outcome"], "requested");
    until("the relayed errand to start", || {
        f.invocations().into_iter().next()
    });

    // And a stopped dispatcher turns back into the 503, not a hang.
    drop(dispatcher);
    let (code, _) = errand(serve.port, "reshape-me", "again");
    assert_eq!(code, 503);
}

#[test]
fn every_other_non_get_stays_refused() {
    let f = errand_fixture();
    let daemon = Daemon::dispatch(&f);
    for path in ["/api/plans", "/api/board", "/api/status", "/"] {
        let (code, _) = post(daemon.port, path, "{}");
        assert_eq!(code, 405, "POST {path} must still be refused");
    }
}

#[test]
fn the_cli_requests_and_reports_the_outcome() {
    let f = errand_fixture();
    f.write(
        "plans/shipped.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: retired\ntype: initiative\n---\n# Shipped\n",
    );
    let _daemon = Daemon::dispatch(&f);
    until("the dispatcher to write its addr", || {
        std::fs::read_to_string(f.root().join(".trellis/runtime/dispatch.addr")).ok()
    });

    let out = f
        .bin()
        .args([
            "dispatch",
            "request",
            "reshape-me",
            "tighten the objective",
            "--model",
            "fable",
        ])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(
        stdout.contains("requested: errand plans/reshape-me.md → org/founder"),
        "{stdout}"
    );
    assert!(stdout.contains("fable"), "the chosen size is reported: {stdout}");

    let out = f
        .bin()
        .args(["dispatch", "request", "plans/shipped.md", "split it"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "a refusal must exit nonzero");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("terminal tier"), "{stderr}");
}

// --- raw HTTP, the same shape serve_inbox.rs uses ---------------------------

fn request(port: u16, method: &str, path: &str, body: &str) -> (u16, String) {
    use std::io::Write;
    use std::net::TcpStream;

    let mut stream = TcpStream::connect(("127.0.0.1", port)).unwrap();
    write!(
        stream,
        "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\
         Content-Type: application/json\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    )
    .unwrap();
    let mut raw = Vec::new();
    stream.read_to_end(&mut raw).unwrap();
    let text = String::from_utf8_lossy(&raw).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").expect("a complete response");
    let code = head
        .lines()
        .next()
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|c| c.parse().ok())
        .expect("a status line");
    (code, body.to_string())
}

fn post(port: u16, path: &str, body: &str) -> (u16, String) {
    request(port, "POST", path, body)
}

fn get(port: u16, path: &str) -> (u16, serde_json::Value) {
    let (code, text) = request(port, "GET", path, "");
    (code, serde_json::from_str(&text).unwrap_or(serde_json::Value::Null))
}
