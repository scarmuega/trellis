//! The back-channel: a session asks, an operator answers, the session
//! continues (decision 0041).
//!
//! `trellis inbox` is the contract surface, so it is what these assertions go
//! through. The raw route is covered too, because the board uses it — but if
//! only one of them worked, it had better be the command.

mod common;

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use common::{Fixture, ANCHOR};

// The subdomain keeps ready fixtures past the scan's mechanical readiness
// precheck (decision 0045).
const FM: &str =
    "---\nprovenance: authored\nowner: org/founder\nsubdomains: [problem/outdoor-retail-channel.md]\n";

/// A live daemon that actually spawns, unlike `serve_api.rs`'s — the whole
/// subject here is what a running session does.
struct Daemon {
    child: Child,
    port: u16,
}

impl Daemon {
    fn start(f: &Fixture) -> Daemon {
        let mut child = Command::new(Fixture::bin_path())
            .args(["serve", "--port", "0", "--tick-secs", "3600"])
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
                    if let Some(addr) = line.strip_prefix("listening on ") {
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
}

impl Drop for Daemon {
    fn drop(&mut self) {
        // `SIGTERM`, not `kill()`: the sessions are in their own process
        // groups, so a `SIGKILL` to the daemon would leave them polling a
        // dead port. Going through the daemon's own stop path both cleans up
        // and exercises it.
        let _ = Command::new("kill")
            .arg(self.child.id().to_string())
            .status();
        let _ = self.child.wait();
    }
}

/// A root whose sessions use the back-channel, with one ready plan to dispatch.
fn asking_fixture() -> Fixture {
    let f = Fixture::healthy();
    let harness = f.asking_harness();
    f.write(
        "runtime.toml",
        &format!(
            "[harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\", \"--mcp-config\", \"{{mcp}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\", \"--mcp-config\", \"{{mcp}}\"]\n"
        ),
    );
    f.write(
        "plans/split-billing.md",
        &format!("{FM}status: ready\ntype: initiative\n---\n# Split billing\n"),
    );
    f
}

/// `trellis inbox --format json`, against the fixture's own root.
fn inbox(f: &Fixture, args: &[&str]) -> serde_json::Value {
    let out = f
        .bin()
        .args(["inbox", "--format", "json"])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "trellis inbox failed: {}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap_or(serde_json::Value::Null)
}

/// Poll until the predicate holds, or give up loudly. Sessions are real
/// processes here, so everything is eventually-consistent by nature.
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
fn a_session_asks_the_operator_answers_through_the_inbox_and_the_session_continues() {
    let f = asking_fixture();
    let _daemon = Daemon::start(&f);

    // The contract surface sees the question, with the options the session
    // offered — that is what makes an answer one keystroke.
    let question = until("the session to ask", || {
        let open = inbox(&f, &[]);
        open.as_array().and_then(|a| a.first().cloned())
    });
    assert_eq!(
        question["label"],
        "act plans/split-billing.md → org/founder"
    );
    assert_eq!(question["question"], "does the table move?");
    assert_eq!(question["context"], "plans/x.md");
    assert_eq!(
        question["options"],
        serde_json::json!(["moves with billing", "stays in core"])
    );

    // Answering by option number rather than by retyping it.
    let ticket = question["ticket"].as_str().unwrap();
    let answered = inbox(&f, &["answer", ticket, "1"]);
    assert_eq!(answered["answer"], "moves with billing");

    // And the session was actually told.
    let got = until("the session to receive the answer", || f.answer_received());
    assert_eq!(got, "moves with billing");
    assert!(
        inbox(&f, &[]).as_array().unwrap().is_empty(),
        "an answered question leaves the inbox"
    );
}

#[test]
fn progress_is_visible_while_the_session_runs_without_opening_its_log() {
    let f = asking_fixture();
    let daemon = Daemon::start(&f);

    let note = until("the session to report progress", || {
        let status = get(daemon.port, "/api/status");
        status["sessions"]
            .as_array()?
            .iter()
            .find_map(|s| s["progress"].as_array()?.first().cloned())
    });
    assert_eq!(note["note"], "claimed and reading");
}

#[test]
fn a_ritual_session_can_ask_too_which_is_why_the_board_is_not_the_contract() {
    let f = Fixture::healthy();
    let harness = f.asking_harness();
    f.write(
        "rituals.md",
        "---\nprovenance: authored\nowner: org/founder\n---\n# Rituals\n\n\
         | ritual           | cadence | executor    | procedure              |\n\
         |------------------|---------|-------------|------------------------|\n\
         | conventions lint | weekly  | org/steward | run the lint checklist |\n",
    );
    // A ritual has no plan card to carry a question on, so it is the case the
    // board structurally cannot serve. Slots are raised because every session
    // here parks on its question until answered, and a starved ritual would
    // look like a channel that does not work.
    f.write(
        "runtime.toml",
        &format!(
            "[scheduler]\nmax_concurrent = 8\n\n\
             [harness]\n\
             act_cmd = [\"{harness}\", \"{{plan}}\", \"--mcp-config\", \"{{mcp}}\"]\n\
             ritual_cmd = [\"{harness}\", \"{{ritual}}\", \"--mcp-config\", \"{{mcp}}\"]\n"
        ),
    );
    let _daemon = Daemon::start(&f);

    let question = until("a ritual to ask", || {
        inbox(&f, &[])
            .as_array()?
            .iter()
            .find(|q| q["key"].as_str().is_some_and(|k| k.starts_with("ritual:")))
            .cloned()
    });
    let ticket = question["ticket"].as_str().unwrap();
    inbox(&f, &["answer", ticket, "stays in core"]);
    let got = until("the ritual to receive the answer", || f.answer_received());
    assert_eq!(got, "stays in core");
}

#[test]
fn the_answer_route_works_raw_and_everything_else_is_still_refused() {
    let f = asking_fixture();
    let daemon = Daemon::start(&f);

    let ticket = until("the session to ask", || {
        let status = get(daemon.port, "/api/status");
        status["pending"].as_array()?.first()?["ticket"]
            .as_str()
            .map(str::to_string)
    });

    // The board's path: a raw POST with the answer in the body.
    let (code, _) = post(
        daemon.port,
        &format!("/api/sessions/{ticket}/answer"),
        r#"{"answer":"stays in core"}"#,
    );
    assert_eq!(code, 200);
    let got = until("the session to receive the answer", || f.answer_received());
    assert_eq!(got, "stays in core");

    // Answering it again is a 404, not a silent overwrite.
    let (code, _) = post(
        daemon.port,
        &format!("/api/sessions/{ticket}/answer"),
        r#"{"answer":"too late"}"#,
    );
    assert_eq!(code, 404);

    // And the read-only property still holds everywhere it held before.
    for path in ["/api/plans", "/api/board", "/api/status", "/"] {
        let (code, _) = post(daemon.port, path, "{}");
        assert_eq!(code, 405, "POST {path} must still be refused");
    }
    let (code, _) = post(daemon.port, "/mcp/not-a-real-token", "{}");
    assert_eq!(code, 404, "an unknown session token is not a door");
}

// --- raw HTTP, the same shape serve_api.rs uses -----------------------------

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

fn get(port: u16, path: &str) -> serde_json::Value {
    let (code, body) = request(port, "GET", path, "");
    assert_eq!(code, 200, "GET {path} → {code}: {body}");
    serde_json::from_str(&body).unwrap_or(serde_json::Value::Null)
}

fn post(port: u16, path: &str, body: &str) -> (u16, String) {
    request(port, "POST", path, body)
}
