//! The serving surface. Every assertion here is the same question: does the
//! API say what the command says? A UI that disagreed with `trellis plan
//! list` would be a second encoding of the domain, which is the drift the
//! kernel exists to prevent.

mod common;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};

use common::{Fixture, ANCHOR};

/// A `trellis serve` running in the background, killed when it goes out of
/// scope. Started on port 0, so tests never collide over a fixed port.
/// Read-only by construction (decision 0046): it loads the tree and serves —
/// dispatching is `serve_dispatch.rs`'s subject, under its own command.
struct Daemon {
    child: Child,
    port: u16,
}

impl Daemon {
    fn start(f: &Fixture) -> Daemon {
        let mut child = Command::new(Fixture::bin_path())
            .args(["serve", "--port", "0"])
            .current_dir(f.root())
            .env("TRELLIS_TODAY", ANCHOR)
            .env_remove("CLAUDE_PLUGIN_ROOT")
            .env("HERDR_SOCKET_PATH", f.root().join(".trellis/no-herdr.sock"))
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
        // Keep draining stdout so a full pipe can never stall the daemon.
        std::thread::spawn(move || lines.for_each(drop));
        Daemon { child, port }
    }

    fn request(&self, method: &str, path: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
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

    fn get(&self, path: &str) -> (u16, String) {
        self.request("GET", path)
    }

    fn json(&self, path: &str) -> serde_json::Value {
        let (code, body) = self.get(path);
        assert_eq!(code, 200, "GET {path} → {code}: {body}");
        serde_json::from_str(&body)
            .unwrap_or_else(|e| panic!("GET {path} is not JSON: {e}\n{body}"))
    }

}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

// The subdomain keeps ready fixtures past the scan's mechanical readiness
// precheck (decision 0045).
const FM: &str =
    "---\nprovenance: authored\nowner: org/founder\nsubdomains: [problem/outdoor-retail-channel.md]\n";
const RECORD: &str = "
## Escalations

```yaml
raised: 2026-08-01
by: org/founder
to: org/founder
status: open
asks: decide whether the second region waits for restock capacity
```
";

fn fixture() -> Fixture {
    let f = Fixture::healthy();
    // Flags that name no plugin checkout, so the daemon starts without one;
    // nothing here is meant to actually run a session.
    // The server outlives this handle: the listener thread owns its socket.
    common::wire_herdr(&f, "\"{plan}\"", "", &["working", "idle"], None);
    f.write(
        "plans/ship-it.md",
        &format!("{FM}status: ready\ntype: initiative\ncomplexity: deep\n---\n# Ship\n"),
    );
    f.write(
        "plans/waits.md",
        &format!("{FM}status: ready\ntype: initiative\nawaits: [plans/ship-it.md]\n---\n# Waits\n"),
    );
    f.write(
        "plans/stuck.md",
        &format!("{FM}status: blocked\ntype: initiative\n---\n# Stuck\n{RECORD}"),
    );
    f.commit_at(common::BACKDATE, "fixture plans");
    f
}

fn cli_json(f: &Fixture, args: &[&str]) -> serde_json::Value {
    let out = f
        .bin()
        .args(args)
        .args(["--format", "json"])
        .output()
        .unwrap();
    serde_json::from_slice(&out.stdout).unwrap()
}

#[test]
fn the_plan_census_matches_the_command_exactly() {
    let f = fixture();
    let d = Daemon::start(&f);
    assert_eq!(d.json("/api/plans"), cli_json(&f, &["plan", "list"]));
}

/// The tier leaves attention (spec rule 13), and both read routes are
/// attention surfaces — so they cut the census the way the command cuts it,
/// and `?archived=1` is `--archived` spelled for a URL. Matching the command
/// is the assertion that matters: it is what keeps an operator reading the
/// CLI and a UI reading the API on one domain.
#[test]
fn the_served_census_sheds_the_terminal_tier_unless_asked() {
    let f = fixture();
    f.write(
        "archive/plans/done.md",
        &format!("{FM}status: retired\ntype: initiative\n---\n# Done\n"),
    );
    f.commit_at(common::BACKDATE, "file a finished plan");
    let d = Daemon::start(&f);

    let live = d.json("/api/plans");
    let plans = |v: &serde_json::Value| -> Vec<String> {
        v.as_array()
            .unwrap()
            .iter()
            .map(|r| r["plan"].as_str().unwrap().to_string())
            .collect()
    };
    assert!(
        !plans(&live).contains(&"archive/plans/done.md".to_string()),
        "a filed plan is off the board: {:?}",
        plans(&live)
    );
    assert_eq!(live, cli_json(&f, &["plan", "list"]));

    let all = d.json("/api/plans?archived=1");
    assert!(
        plans(&all).contains(&"archive/plans/done.md".to_string()),
        "asking for the tier gets it: {:?}",
        plans(&all)
    );
    assert_eq!(all, cli_json(&f, &["plan", "list", "--archived"]));

    // The board is the same cut, one projection further on.
    let retired = d.json("/api/board")["columns"]["retired"].clone();
    assert!(
        retired.as_array().unwrap().is_empty(),
        "the retired column is live plans only: {retired}"
    );
}

#[test]
fn one_plans_facts_match_trellis_show() {
    let f = fixture();
    let d = Daemon::start(&f);
    assert_eq!(
        d.json("/api/plans/ship-it"),
        cli_json(&f, &["show", "plans/ship-it.md"])
    );
    assert_eq!(
        d.json("/api/plans/ship-it.md"),
        cli_json(&f, &["show", "plans/ship-it.md"]),
        "the .md is optional in the path"
    );
}

#[test]
fn the_board_partitions_every_plan_and_keeps_a_hold_off_the_columns() {
    let f = fixture();
    let d = Daemon::start(&f);
    let board = d.json("/api/board");
    let all = d.json("/api/plans");

    let placed: usize = board["columns"]
        .as_object()
        .unwrap()
        .values()
        .map(|c| c.as_array().unwrap().len())
        .sum();
    assert_eq!(
        placed + board["unplaced"].as_array().unwrap().len(),
        all.as_array().unwrap().len(),
        "every plan lands in exactly one column"
    );

    let ready = board["columns"]["ready"].as_array().unwrap();
    let waits = ready
        .iter()
        .find(|p| p["plan"] == "plans/waits.md")
        .expect("a held plan is still ready");
    assert_eq!(waits["held"], "plans/ship-it.md", "the hold rides the card");
    assert_eq!(board["order"][1], "ready");
}

#[test]
fn open_escalations_are_served_as_the_command_lists_them() {
    let f = fixture();
    let d = Daemon::start(&f);
    let records = d.json("/api/escalations");
    assert_eq!(records, cli_json(&f, &["escalate", "list"]));
    assert_eq!(records[0]["artifact"], "plans/stuck.md");
    assert_eq!(records[0]["status"], "open");

    let plans = d.json("/api/plans");
    let stuck = plans
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["plan"] == "plans/stuck.md")
        .unwrap();
    assert_eq!(stuck["open_escalations"], 1, "the board card can show it");
}

#[test]
fn the_dispatch_endpoint_reports_what_the_daemon_would_start() {
    let f = fixture();
    let d = Daemon::start(&f);
    let report = d.json("/api/dispatch");
    assert_eq!(report["dispatch"][0]["plan"], "plans/ship-it.md");
    assert_eq!(
        report["dispatch"][0]["model"], "fable",
        "the deep session this daemon is configured with, not a stock default"
    );
    assert_eq!(report["held"][0]["plan"], "plans/waits.md");
    assert!(
        f.read("plans/ship-it.md").contains("status: ready"),
        "reading the scan is not running it"
    );
}

#[test]
fn views_are_served_as_the_markdown_they_are() {
    let f = fixture();
    let d = Daemon::start(&f);
    let (code, body) = d.get("/api/views/board");
    assert_eq!(code, 200);
    assert!(body.contains("provenance: generated"), "{body}");
    assert!(body.contains("generated-by: trellis view board"), "{body}");
    assert_eq!(d.get("/api/views/nonesuch").0, 404);
}

#[test]
fn artifacts_are_served_by_their_root_relative_path() {
    let f = fixture();
    let d = Daemon::start(&f);
    let artifact = d.json("/api/artifacts/plans/ship-it.md");
    assert_eq!(artifact["path"], "plans/ship-it.md");
    assert_eq!(artifact["kind"], "plan");
    assert!(artifact["text"].as_str().unwrap().contains("# Ship"));
    assert_eq!(artifact["facts"]["complexity"], "deep");
}

#[test]
fn nothing_outside_the_tree_is_reachable() {
    let f = fixture();
    let d = Daemon::start(&f);
    for path in [
        "/api/artifacts/../../../etc/passwd",
        "/api/artifacts/.trellis/runtime/state.json",
        "/api/artifacts/runtime.toml",
        "/api/artifacts/trellis.toml",
        "/api/plans/nonesuch",
        "/etc/passwd",
    ] {
        assert_eq!(d.get(path).0, 404, "{path} must not resolve");
    }
    // The prose half of the split is an ordinary artifact and serves.
    let artifact = d.json("/api/artifacts/domain.md");
    assert_eq!(artifact["kind"], "domain");
}

#[test]
fn the_surface_is_read_only() {
    let f = fixture();
    let d = Daemon::start(&f);
    let (code, body) = d.request("POST", "/api/plans");
    assert_eq!(code, 405);
    assert!(body.contains("read-only"), "{body}");
}

/// The read-only server has no pass of its own (decision 0046): it overlays
/// the dispatcher's `status.json` snapshot and says whether that process is
/// still running.
#[test]
fn the_server_reports_itself_and_overlays_the_dispatchers_snapshot() {
    let f = fixture();
    // A real dispatch pass first, so there is a snapshot to serve. The
    // claiming harness flips ship-it ready → active; waits.md stays held.
    f.dispatch_once(ANCHOR, &[]);
    let d = Daemon::start(&f);
    let status = d.json("/api/status");
    assert_eq!(status["port"], d.port);
    assert_eq!(status["process"], "serve");
    assert!(status["pid"].as_u64().unwrap() > 0);
    assert!(status["root"]
        .as_str()
        .unwrap()
        .ends_with(f.root().file_name().unwrap().to_str().unwrap()));
    assert_eq!(status["dispatch"]["process"], "dispatch");
    assert_eq!(status["dispatch"]["last_pass"], ANCHOR);
    assert_eq!(status["dispatch"]["held"][0], "plans/waits.md");
    assert_eq!(
        status["dispatch_running"], false,
        "the --once pass exited; the server says so instead of pretending"
    );
}

#[test]
fn the_board_page_is_served_at_the_root() {
    let f = fixture();
    let d = Daemon::start(&f);
    let (code, body) = d.get("/");
    assert_eq!(code, 200);
    assert!(body.contains("<title>Trellis board</title>"), "{body}");
    assert!(body.contains("/api/board"), "the page reads the API");
}
