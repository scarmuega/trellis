//! Status flips over HTTP: the operator's lifecycle verbs behind the window
//! (decision 0049). The route performs the same guarded flip the CLI verbs
//! do — readiness gates release, holds gate claim, terminal stays terminal —
//! honored on the dispatcher's socket and relayed by serve.

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

fn flip(port: u16, slug: &str, body: &str) -> (u16, serde_json::Value) {
    let (code, text) = request(port, "POST", &format!("/api/plans/{slug}/status"), body);
    let value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    (code, value)
}

fn status_of(f: &Fixture, rel: &str) -> String {
    f.read(rel)
        .lines()
        .find_map(|l| l.strip_prefix("status:").map(|s| s.trim().to_string()))
        .expect("a status: line")
}

/// A root whose plans declare no `subdomains:`, so they fail the mechanical
/// readiness share — which keeps the scan from ever dispatching them and
/// makes the release gate observable, both at once.
fn flipping_fixture() -> Fixture {
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
        "plans/still-drafting.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: draft\ntype: initiative\n---\n# Still drafting\n",
    );
    f
}

#[test]
fn release_gates_on_readiness_and_force_flips() {
    let f = flipping_fixture();
    let daemon = Daemon::dispatch(&f);

    // No subdomains: → readiness item 4 fails → the gate holds the draft.
    let (code, v) = flip(daemon.port, "still-drafting", r#"{"status": "ready"}"#);
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "not-ready");
    assert!(v["error"].as_str().unwrap().contains("readiness"), "{v}");
    assert_eq!(status_of(&f, "plans/still-drafting.md"), "draft");

    // Force is the same override the CLI offers, spelled in the body.
    let (code, v) = flip(
        daemon.port,
        "still-drafting",
        r#"{"status": "ready", "force": true}"#,
    );
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["outcome"], "flipped");
    assert_eq!(v["from"], "draft");
    assert_eq!(v["to"], "ready");
    assert_eq!(status_of(&f, "plans/still-drafting.md"), "ready");
}

#[test]
fn claim_respects_holds_and_retire_ends_anything() {
    let f = flipping_fixture();
    f.write(
        "plans/prerequisite.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: active\ntype: initiative\n---\n# Prerequisite\n",
    );
    f.write(
        "plans/waiting.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: ready\ntype: initiative\nawaits: [plans/prerequisite.md]\n---\n# Waiting\n",
    );
    f.write(
        "plans/free.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: ready\ntype: initiative\n---\n# Free\n",
    );
    let daemon = Daemon::dispatch(&f);

    let (code, v) = flip(daemon.port, "waiting", r#"{"status": "active"}"#);
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "held");
    assert!(
        v["error"].as_str().unwrap().contains("plans/prerequisite.md"),
        "{v}"
    );

    let (code, v) = flip(daemon.port, "free", r#"{"status": "active"}"#);
    assert_eq!(code, 200, "{v}");
    assert_eq!(status_of(&f, "plans/free.md"), "active");

    let (code, v) = flip(daemon.port, "free", r#"{"status": "retired"}"#);
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["from"], "active");
    assert_eq!(status_of(&f, "plans/free.md"), "retired");

    // Retirement reaches a draft too — abandoning is a legal end.
    let (code, v) = flip(daemon.port, "still-drafting", r#"{"status": "retired"}"#);
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["from"], "draft");
}

#[test]
fn illegal_flips_speak_the_refusal_vocabulary() {
    let f = flipping_fixture();
    f.write(
        "plans/shipped.md",
        "---\nprovenance: authored\nowner: org/founder\nstatus: retired\ntype: initiative\n---\n# Shipped\n",
    );
    let daemon = Daemon::dispatch(&f);

    let (code, v) = flip(daemon.port, "never-heard-of-it", r#"{"status": "ready"}"#);
    assert_eq!(code, 404, "{v}");
    assert_eq!(v["outcome"], "not-a-plan");

    // A draft cannot be claimed — active starts from ready.
    let (code, v) = flip(daemon.port, "still-drafting", r#"{"status": "active"}"#);
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "illegal");

    // Retired is terminal: nothing flips it back.
    let (code, v) = flip(daemon.port, "shipped", r#"{"status": "ready"}"#);
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "illegal");

    // Targets no verb reaches, and targets that are not statuses at all.
    let (code, _) = flip(daemon.port, "still-drafting", r#"{"status": "draft"}"#);
    assert_eq!(code, 400);
    let (code, v) = flip(daemon.port, "still-drafting", r#"{"status": "blocked"}"#);
    assert_eq!(code, 400);
    assert!(v["error"].as_str().unwrap().contains("trellis plan block"), "{v}");
    let (code, _) = flip(daemon.port, "still-drafting", r#"{"status": "launched"}"#);
    assert_eq!(code, 400);
    let (code, _) = flip(daemon.port, "still-drafting", "{}");
    assert_eq!(code, 400);
}

#[test]
fn a_flip_under_a_running_session_is_refused() {
    let f = flipping_fixture();
    // A harness that outlives the check, so the refine session it carries is
    // still in flight when the flip arrives.
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
    let daemon = Daemon::dispatch(&f);

    let body = serde_json::json!({ "instruction": "split the scope" }).to_string();
    let (code, _) = request(
        daemon.port,
        "POST",
        "/api/plans/still-drafting/refine",
        &body,
    );
    assert_eq!(code, 200);
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let (_, text) = request(daemon.port, "GET", "/api/status", "");
        let status: serde_json::Value = serde_json::from_str(&text).unwrap();
        if status["in_flight"]
            .as_array()
            .map(|a| a.iter().any(|s| s["key"] == "plan:plans/still-drafting.md"))
            .unwrap_or(false)
        {
            break;
        }
        assert!(Instant::now() < deadline, "the session never showed in flight");
        std::thread::sleep(Duration::from_millis(100));
    }

    let (code, v) = flip(
        daemon.port,
        "still-drafting",
        r#"{"status": "retired"}"#,
    );
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "in-flight");
    assert_eq!(status_of(&f, "plans/still-drafting.md"), "draft");
}

#[test]
fn serve_relays_the_flip_and_names_the_fix_alone() {
    let f = flipping_fixture();
    let serve = Daemon::serve(&f);

    // No dispatcher: serve refuses to pretend, and names the fix.
    let (code, v) = flip(serve.port, "still-drafting", r#"{"status": "retired"}"#);
    assert_eq!(code, 503, "{v}");
    assert!(
        v["error"].as_str().unwrap().contains("trellis dispatch run"),
        "{v}"
    );
    assert_eq!(status_of(&f, "plans/still-drafting.md"), "draft");

    // With one running, the flip lands through serve's port — refusals and
    // successes both come back verbatim.
    let _dispatch = Daemon::dispatch(&f);
    let (code, v) = flip(serve.port, "still-drafting", r#"{"status": "active"}"#);
    assert_eq!(code, 409, "{v}");
    assert_eq!(v["outcome"], "illegal");
    let (code, v) = flip(serve.port, "still-drafting", r#"{"status": "retired"}"#);
    assert_eq!(code, 200, "{v}");
    assert_eq!(v["outcome"], "flipped");
    assert_eq!(status_of(&f, "plans/still-drafting.md"), "retired");
}
