#![allow(dead_code)] // shared across test binaries; not every bin uses every helper

//! Fixture harness: ports `evals/run.sh`'s layer composition (skeleton +
//! overlay + fixture delta) and `evals/prepare.mjs`'s date-token
//! substitution, anchored to a FIXED date via `TRELLIS_TODAY` so dwell
//! arithmetic and rendered views are hermetic.

use std::path::{Path, PathBuf};
use std::process::Command;

use trellis::dates::Date;

/// Every test runs "today = 2026-08-03".
pub const ANCHOR: &str = "2026-08-03";
/// The composing commit is backdated well before the anchor.
pub const BACKDATE: &str = "2026-06-24";

pub fn repo_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(rel)
}

pub struct Fixture {
    pub dir: tempfile::TempDir,
}

fn anchor_date() -> Date {
    Date::parse(ANCHOR).unwrap()
}

fn substitute_tokens(text: &str) -> String {
    let anchor = anchor_date().days_from_epoch();
    let iso = |days: i64| Date::from_epoch_days(days).to_string();
    let mut out = text.replace("%%TODAY%%", &iso(anchor));
    let ago = regex_lite(r"%%DAYS_AGO_(\d+)%%");
    while let Some((full, n)) = ago(&out) {
        out = out.replacen(&full, &iso(anchor - n), 1);
    }
    let ahead = regex_lite(r"%%DAYS_AHEAD_(\d+)%%");
    while let Some((full, n)) = ahead(&out) {
        out = out.replacen(&full, &iso(anchor + n), 1);
    }
    out
}

/// Tiny matcher: returns (full match, captured number) for the first hit.
fn regex_lite(pattern: &str) -> impl Fn(&str) -> Option<(String, i64)> + '_ {
    let re = regex::Regex::new(pattern).unwrap();
    move |s: &str| {
        re.captures(s)
            .map(|c| (c[0].to_string(), c[1].parse().unwrap()))
    }
}

fn copy_layer(src: &Path, dest: &Path) {
    for entry in walkdir::WalkDir::new(src).sort_by_file_name() {
        let entry = entry.unwrap();
        let rel = entry.path().strip_prefix(src).unwrap();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target).unwrap();
        } else {
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            let content = std::fs::read_to_string(entry.path())
                .map(|t| substitute_tokens(&t))
                .map(String::into_bytes)
                .unwrap_or_else(|_| std::fs::read(entry.path()).unwrap());
            std::fs::write(&target, content).unwrap();
        }
    }
}

impl Fixture {
    pub fn compose(layers: &[&str]) -> Fixture {
        let dir = tempfile::TempDir::new().unwrap();
        for layer in layers {
            copy_layer(&repo_path(layer), dir.path());
        }
        let f = Fixture { dir };
        f.git(&["init", "-q"]);
        f.git(&["config", "user.email", "fixture@example.invalid"]);
        f.git(&["config", "user.name", "fixture"]);
        f.commit_at(BACKDATE, "compose fixture");
        f
    }

    pub fn healthy() -> Fixture {
        Fixture::compose(&["evals/skeleton", "evals/fixtures/healthy"])
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn git(&self, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    pub fn commit_at(&self, date: &str, msg: &str) {
        self.git(&["add", "-A"]);
        let stamp = format!("{date}T09:00:00Z");
        let out = Command::new("git")
            .arg("-C")
            .arg(self.root())
            .args(["commit", "-q", "--allow-empty", "-m", msg])
            .env("GIT_AUTHOR_DATE", &stamp)
            .env("GIT_COMMITTER_DATE", &stamp)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    pub fn write(&self, rel: &str, content: &str) {
        let path = self.root().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, substitute_tokens(content)).unwrap();
    }

    pub fn read(&self, rel: &str) -> String {
        std::fs::read_to_string(self.root().join(rel)).unwrap()
    }

    /// The trellis binary, cwd at the fixture root, hermetic today.
    pub fn bin(&self) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("trellis").unwrap();
        cmd.current_dir(self.root())
            .env("TRELLIS_TODAY", ANCHOR)
            .env_remove("CLAUDE_PLUGIN_ROOT");
        cmd
    }

    /// A stand-in for `claude`: records the argv it was handed, one argument
    /// per line, and — when its first argument names a plan file — claims it
    /// the way a real taker does, by flipping `ready → active`. Lives under
    /// `.trellis/`, which the tree walker skips, so it is never an artifact.
    ///
    /// Every serve test runs against this rather than a harness, which is the
    /// point of the command template: the reference adapter is configuration,
    /// so a hermetic one is too.
    pub fn fake_harness(&self) -> String {
        let rel = ".trellis/bin/harness";
        let path = self.root().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // One file per invocation, named by second and pid. Sessions run
        // concurrently and a shared append log interleaves their lines —
        // shell printf is no more atomic than a sequence of echoes.
        std::fs::write(
            &path,
            "#!/bin/sh\n\
             dir=.trellis/runtime/argv\n\
             mkdir -p \"$dir\"\n\
             for a in \"$@\"; do echo \"$a\"; done > \"$dir/$(date +%s)-$$\"\n\
             # Snapshot the runtime-stamped acting-role marker (decision\n\
             # 0045): it exists only while sessions run, so the assertion\n\
             # has to be taken from inside one.\n\
             if [ -f .trellis/acting-role ]; then\n\
             \x20 cp .trellis/acting-role \"$dir/../marker-seen-$$\"\n\
             fi\n\
             if [ -n \"$1\" ] && [ -f \"$1\" ]; then\n\
             \x20 sed 's/^status: ready$/status: active/' \"$1\" > \"$1.claimed\" \\\n\
             \x20   && mv \"$1.claimed\" \"$1\"\n\
             fi\n\
             exit 0\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        rel.to_string()
    }

    /// A stand-in for `claude` that uses the back-channel: it reports
    /// progress, asks a question, and waits for the answer, writing whatever
    /// it is told to `.trellis/runtime/answer`.
    ///
    /// It reads the endpoint out of its own `--mcp-config` argument, which is
    /// what a real harness does too — the daemon hands the session its
    /// channel the same way either way. The whole exchange is `curl` against
    /// `/mcp/{token}`, which is the check that matters: the reference adapter
    /// is configuration, so a hermetic one still is.
    pub fn asking_harness(&self) -> String {
        let rel = ".trellis/bin/asking";
        let path = self.root().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            r##"#!/bin/sh
dir=.trellis/runtime
mkdir -p "$dir/argv"
for a in "$@"; do echo "$a"; done > "$dir/argv/$(date +%s)-$$"

# The endpoint, out of the --mcp-config value this session was handed.
url=
prev=
for a in "$@"; do
  if [ "$prev" = "--mcp-config" ]; then
    url=$(printf '%s' "$a" | sed -n 's|.*"url" *: *"\([^"]*\)".*|\1|p')
  fi
  prev=$a
done
[ -n "$url" ] || { echo "no mcp endpoint in argv" >&2; exit 9; }

rpc() { curl -sS -m 10 -H 'Content-Type: application/json' -d "$1" "$url"; }

rpc '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"trellis_progress","arguments":{"note":"claimed and reading"}}}' >/dev/null

asked=$(rpc '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"trellis_ask","arguments":{"question":"does the table move?","options":["moves with billing","stays in core"],"context":"plans/x.md"}}}')
ticket=$(printf '%s' "$asked" | sed -n 's|.*ticket \([0-9a-f]*\) .*|\1|p')
[ -n "$ticket" ] || { echo "no ticket in: $asked" >&2; exit 9; }

# Poll the bounded wait, the way an agent that is willing to wait does.
i=0
while [ "$i" -lt 30 ]; do
  got=$(rpc "{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"trellis_await\",\"arguments\":{\"ticket\":\"$ticket\"}}}")
  answer=$(printf '%s' "$got" | sed -n 's|.*answered: \([^"]*\)".*|\1|p')
  if [ -n "$answer" ]; then
    printf '%s' "$answer" > "$dir/answer"
    exit 0
  fi
  i=$((i + 1))
done
echo "never answered" >&2
exit 9
"##,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        rel.to_string()
    }

    /// What the asking harness was told, once it has been told.
    pub fn answer_received(&self) -> Option<String> {
        std::fs::read_to_string(self.root().join(".trellis/runtime/answer")).ok()
    }

    /// Every invocation the fake harness recorded, as argv vectors. Ordered
    /// by the second each session started, so passes are in order and
    /// sessions within one pass are not — which is all the daemon promises.
    pub fn invocations(&self) -> Vec<Vec<String>> {
        let dir = self.root().join(".trellis/runtime/argv");
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
        files.sort();
        files
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
            .map(|text| text.lines().map(str::to_string).collect())
            .collect()
    }

    /// The dispatcher's state file.
    pub fn state(&self) -> serde_json::Value {
        let path = self.root().join(".trellis/runtime/dispatch-state.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(serde_json::Value::Null)
    }

    /// The rituals state file.
    pub fn rituals_state(&self) -> serde_json::Value {
        let path = self.root().join(".trellis/runtime/rituals-state.json");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or(serde_json::Value::Null)
    }

    /// `trellis dispatch run --once …` at a given date, returning its stdout.
    pub fn dispatch_once(&self, today: &str, args: &[&str]) -> String {
        let out = self
            .bin()
            .env("TRELLIS_TODAY", today)
            .args(["dispatch", "run", "--once"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "dispatch run --once failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// `trellis rituals …` at a given date, returning its stdout.
    pub fn rituals_once(&self, today: &str, args: &[&str]) -> String {
        let out = self
            .bin()
            .env("TRELLIS_TODAY", today)
            .args(["rituals"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "rituals failed: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).into_owned()
    }

    /// `trellis rituals …` expected to refuse: stdout+stderr, asserted nonzero.
    #[allow(dead_code)]
    pub fn rituals_expect_failure(&self, today: &str, args: &[&str]) -> String {
        let out = self
            .bin()
            .env("TRELLIS_TODAY", today)
            .args(["rituals"])
            .args(args)
            .output()
            .unwrap();
        assert!(
            !out.status.success(),
            "rituals succeeded where a refusal was owed: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    }

    /// The built binary, for the cases that must not block on completion.
    pub fn bin_path() -> PathBuf {
        assert_cmd::cargo::cargo_bin("trellis")
    }

    pub fn lint_json(&self, extra: &[&str]) -> serde_json::Value {
        let out = self
            .bin()
            .args(["lint", "--format", "json"])
            .args(extra)
            .output()
            .unwrap();
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
            panic!(
                "lint --format json did not emit JSON: {e}\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        })
    }
}

/// A hermetic herdr server: a `UnixListener` speaking the NDJSON protocol,
/// one request per connection like the real one, recording every request and
/// answering `agent.get`/`agent.wait` from a scripted status sequence. The
/// herdr socket is config the same way the harness is, so a fake server is a
/// test helper the same way `fake_harness` is.
#[cfg(unix)]
pub struct FakeHerdr {
    pub socket: PathBuf,
    requests: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
}

#[cfg(unix)]
impl FakeHerdr {
    /// `statuses` feeds `agent.get`/`agent.wait`, front to back, sticking on
    /// the last; `agents` is what `agent.list` reports (adoption seeds).
    pub fn start(socket: PathBuf, statuses: &[&str], agents: Vec<serde_json::Value>) -> FakeHerdr {
        Self::start_dropping(socket, statuses, agents, 0)
    }

    /// Like `start`, but the first `drops` prompts are accepted and then
    /// swallowed — never echoed into `agent.read` — the way a live Claude
    /// drops injected input while still starting up.
    pub fn start_dropping(
        socket: PathBuf,
        statuses: &[&str],
        agents: Vec<serde_json::Value>,
        drops: usize,
    ) -> FakeHerdr {
        use serde_json::json;
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixListener;

        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        let listener = UnixListener::bind(&socket).unwrap();
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = std::sync::Arc::clone(&requests);
        let mut statuses: std::collections::VecDeque<String> =
            statuses.iter().map(|s| s.to_string()).collect();

        std::thread::spawn(move || {
            let mut counter = 0u64;
            let mut seq = 10u64;
            let mut prompts_seen = 0usize;
            // What agent.read hands back: the echo of every prompt that
            // landed, the way a real pane's transcript carries them.
            let mut scrollback = String::new();
            loop {
                let Ok((stream, _)) = listener.accept() else {
                    return;
                };
                let mut line = String::new();
                if BufReader::new(&stream).read_line(&mut line).is_err() {
                    continue;
                }
                let Ok(request) = serde_json::from_str::<serde_json::Value>(&line) else {
                    continue;
                };
                record.lock().unwrap().push(request.clone());
                let id = request["id"].as_str().unwrap_or("").to_string();
                let method = request["method"].as_str().unwrap_or("");
                let params = &request["params"];

                let agent_at = |pane: &str, status: &str, seq: u64| {
                    json!({
                        "pane_id": pane, "workspace_id": "w-fake", "tab_id": "t1",
                        "terminal_id": "term1", "focused": false, "revision": 1,
                        "agent_status": status, "state_change_seq": seq,
                        "interactive_ready": true,
                    })
                };
                let mut next_status = || {
                    if statuses.len() > 1 {
                        statuses.pop_front().unwrap()
                    } else {
                        statuses.front().cloned().unwrap_or_else(|| "idle".into())
                    }
                };

                let result = match method {
                    "ping" => json!({"type": "pong", "version": "fake", "protocol": 19}),
                    "notification.show" | "workspace.close" | "pane.report_metadata" => {
                        json!({"type": "ok"})
                    }
                    "workspace.create" => {
                        counter += 1;
                        json!({
                            "type": "workspace_created",
                            "workspace": {"workspace_id": format!("w{counter}")},
                            "tab": {},
                            "root_pane": {"pane_id": format!("p{counter}")},
                        })
                    }
                    "agent.start" => {
                        // The real server's name rule, enforced here so the
                        // hermetic tests catch what the live one refuses.
                        let name = params["name"].as_str().unwrap_or("");
                        let legal = name.len() <= 32
                            && name.starts_with(|c: char| c.is_ascii_lowercase())
                            && name.chars().all(|c| {
                                c.is_ascii_lowercase() || c.is_ascii_digit() || "-_".contains(c)
                            });
                        if !legal {
                            let mut writer = &stream;
                            let _ = writeln!(
                                writer,
                                r#"{{"id":"{id}","error":{{"code":"invalid_agent_name","message":"agent name must start with a lowercase letter and contain only lowercase letters, digits, '-' or '_' (1-32 characters)"}}}}"#
                            );
                            continue;
                        }
                        json!({
                            "type": "agent_started", "argv": [],
                            "agent": agent_at(params["pane_id"].as_str().unwrap_or("p?"), "idle", 1),
                        })
                    }
                    "agent.prompt" => {
                        prompts_seen += 1;
                        if prompts_seen > drops {
                            scrollback.push_str(params["text"].as_str().unwrap_or(""));
                            scrollback.push('\n');
                        }
                        json!({
                            "type": "agent_prompted",
                            "agent": agent_at(params["target"].as_str().unwrap_or("p?"), "working", 2),
                        })
                    }
                    "agent.get" | "agent.wait" => {
                        seq += 1;
                        let status = next_status();
                        json!({
                            "type": "agent_info",
                            "agent": agent_at(params["target"].as_str().unwrap_or("p?"), &status, seq),
                        })
                    }
                    "agent.list" => json!({"type": "agent_list", "agents": agents}),
                    "agent.read" => json!({
                        "type": "pane_read",
                        "read": {"text": format!("{scrollback}fake scrollback tail")},
                    }),
                    other => {
                        let mut writer = &stream;
                        let _ = writeln!(
                            writer,
                            r#"{{"id":"{id}","error":{{"code":"invalid_request","message":"fake herdr does not speak {other}"}}}}"#
                        );
                        continue;
                    }
                };
                let mut writer = &stream;
                let _ = writeln!(writer, "{}", json!({"id": id, "result": result}));
            }
        });
        FakeHerdr { socket, requests }
    }

    /// Method names, in arrival order.
    pub fn calls(&self) -> Vec<String> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter_map(|r| r["method"].as_str().map(str::to_string))
            .collect()
    }

    /// The params of every request for one method.
    pub fn params_for(&self, method: &str) -> Vec<serde_json::Value> {
        self.requests
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r["method"] == method)
            .map(|r| r["params"].clone())
            .collect()
    }
}

/// An `agent.list` entry as herdr would report a still-running trellis
/// session — what adoption reads.
#[cfg(unix)]
pub fn herdr_agent(key: &str, root: &Path, label: &str, status: &str) -> serde_json::Value {
    serde_json::json!({
        "pane_id": "p-adopted", "workspace_id": "w-adopted", "tab_id": "t1",
        "terminal_id": "term1", "focused": false, "revision": 1,
        "agent_status": status, "state_change_seq": 5, "interactive_ready": true,
        "tokens": {
            "trellis_key": key,
            "trellis_root": root.display().to_string(),
            "trellis_label": label,
        },
    })
}

pub fn finding_items(report: &serde_json::Value) -> Vec<u8> {
    report["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["item"].as_u64().unwrap() as u8)
        .collect()
}
