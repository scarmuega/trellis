//! Refine: an operator asks a plan's owner to reshape its content, and the
//! dispatch loop — still the only spawner — fires the session (decision
//! 0048). The route is honored locally on the dispatcher's socket, relayed
//! by serve, and every other non-GET stays refused.

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

/// A root with a draft plan to refine. Drafts are refinable and never
/// scanned, so the only sessions these tests see are the ones they asked for.
fn refining_fixture() -> Fixture {
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

fn refine(port: u16, slug: &str, instruction: &str) -> (u16, serde_json::Value) {
    let body = serde_json::json!({ "instruction": instruction }).to_string();
    let (code, text) = post(port, &format!("/api/plans/{slug}/refine"), &body);
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
fn a_refine_request_spawns_on_the_wake_and_carries_the_instruction() {
    let f = refining_fixture();
    let daemon = Daemon::dispatch(&f);

    let (code, outcome) = refine(daemon.port, "reshape-me", "split the scope");
    assert_eq!(code, 200, "{outcome}");
    assert_eq!(outcome["outcome"], "requested");
    assert_eq!(outcome["owner"], "org/founder");
    assert_eq!(outcome["plan"], "plans/reshape-me.md");

    // The tick is an hour away, so only the wake can have started this — and
    // the prompt the harness got is the rendered framework procedure
    // (decision 0050): unique first line, instruction verbatim, the
    // refinement contract and the act body it delegates to.
    let argv = until("the refine session to start", || {
        f.invocations().into_iter().next()
    });
    assert_eq!(argv[0], "refine plans/reshape-me.md as founder (trellis runtime).");
    let prompt = argv.join("\n");
    assert!(prompt.contains("verbatim: split the scope"), "{prompt}");
    assert!(prompt.contains("The refinement contract"), "{prompt}");
    assert!(prompt.contains("Bind to the domain root"), "{prompt}");
}

#[test]
fn refusals_speak_the_tree_vocabulary_and_spawn_nothing() {
    let f = refining_fixture();
    f.write(
        "plans/shipped.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: retired\ntype: initiative\n---\n# Shipped\n",
    );
    f.write(
        "plans/orphan.md",
        "---\nprovenance: authored\nstatus: draft\ntype: initiative\n---\n# Orphan\n",
    );
    f.write(
        "org/advisor/mandate.md",
        "---\nprovenance: authored\nowner: org/founder\npurpose: advise\nescalate-to: org/founder\nholder: holder/\n---\n# Advisor\n",
    );
    f.write(
        "org/advisor/holder/ref.md",
        "---\nprovenance: authored\nowner: org/founder\nref: a human advisor\nkind: human\n---\n# Holder\n",
    );
    f.write(
        "plans/advice.md",
        "---\nprovenance: authored\nowner: org/advisor\nstatus: draft\ntype: initiative\n---\n# Advice\n",
    );
    let daemon = Daemon::dispatch(&f);

    let (code, v) = refine(daemon.port, "never-heard-of-it", "split it");
    assert_eq!(code, 404, "{v}");
    assert_eq!(v["outcome"], "not-a-plan");

    let (code, v) = refine(daemon.port, "shipped", "split it");
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "terminal");

    let (code, v) = refine(daemon.port, "orphan", "split it");
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "unowned");

    let (code, v) = refine(daemon.port, "advice", "split it");
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "handoff");

    let (code, _) = post(daemon.port, "/api/plans/reshape-me/refine", "{}");
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

    let (code, _) = refine(daemon.port, "reshape-me", "split the scope");
    assert_eq!(code, 200);
    until("the session to show in flight", || {
        let (_, status) = get(daemon.port, "/api/status");
        status["in_flight"]
            .as_array()?
            .iter()
            .any(|s| s["key"] == "plan:plans/reshape-me.md")
            .then_some(())
    });

    let (code, v) = refine(daemon.port, "reshape-me", "split it differently");
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "in-flight");
}

#[test]
fn serve_relays_to_the_dispatcher_and_says_so_when_there_is_none() {
    let f = refining_fixture();
    let serve = Daemon::serve(&f);

    // No dispatcher yet: serve refuses to pretend, and names the fix.
    let (code, v) = refine(serve.port, "reshape-me", "split the scope");
    assert_eq!(code, 503, "{v}");
    assert!(
        v["error"].as_str().unwrap().contains("trellis dispatch run"),
        "{v}"
    );

    // With one running, the same POST to *serve* spawns — the board's path.
    let dispatcher = Daemon::dispatch(&f);
    let (code, v) = refine(serve.port, "reshape-me", "split the scope");
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["outcome"], "requested");
    until("the relayed refine to start", || {
        f.invocations().into_iter().next()
    });

    // And a stopped dispatcher turns back into the 503, not a hang.
    drop(dispatcher);
    let (code, _) = refine(serve.port, "reshape-me", "again");
    assert_eq!(code, 503);
}

#[test]
fn every_other_non_get_stays_refused() {
    let f = refining_fixture();
    let daemon = Daemon::dispatch(&f);
    for path in ["/api/plans", "/api/board", "/api/status", "/"] {
        let (code, _) = post(daemon.port, path, "{}");
        assert_eq!(code, 405, "POST {path} must still be refused");
    }
}

#[test]
fn the_cli_requests_and_reports_the_outcome() {
    let f = refining_fixture();
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
        .args(["dispatch", "refine", "reshape-me", "tighten the objective"])
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "{stdout}");
    assert!(
        stdout.contains("requested: plans/reshape-me.md → org/founder"),
        "{stdout}"
    );

    let out = f
        .bin()
        .args(["dispatch", "refine", "plans/shipped.md", "split it"])
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
