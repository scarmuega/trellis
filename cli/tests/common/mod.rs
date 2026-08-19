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
        // A fixture is what the layer declares, never what the machine left
        // lying in it. `.DS_Store` is ignored by most developers' *global*
        // gitignore, so it is invisible to `git status`, absent from a fresh
        // checkout, and — copied in here — silently becomes one more
        // git-ignored path for the scope census to count. That reads as a
        // kernel bug on one machine and passes everywhere else.
        if entry.file_name() == ".DS_Store" {
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
            .env_remove("CLAUDE_PLUGIN_ROOT")
            // Never the operator's own herdr: a fixture that configures no
            // socket must fail to connect, not drive the real fleet.
            .env(
                "HERDR_SOCKET_PATH",
                self.root().join(".trellis/no-herdr.sock"),
            );
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
        self.fake_harness_leaving("harness", "")
    }

    /// `fake_harness`, plus the verdict a taker owes its plan: `verdict` is
    /// sh run after the claim with `"$1"` naming the plan file. The bare
    /// `fake_harness` is this with no verdict at all — a session that claims
    /// the work and walks away, which is the case the runtime has to catch.
    ///
    /// The claim is a no-op since decision 0053 — the runtime takes it before
    /// the session exists — and is kept because it must stay one: a harness
    /// that still claims is exactly what a live session running the old
    /// contract would do.
    pub fn fake_harness_leaving(&self, name: &str, verdict: &str) -> String {
        let rel = format!(".trellis/bin/{name}");
        let path = self.root().join(&rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // One file per invocation, named by second and pid. Sessions run
        // concurrently and a shared append log interleaves their lines —
        // shell printf is no more atomic than a sequence of echoes.
        std::fs::write(
            &path,
            format!(
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
                 # Snapshot the plan as the session found it, before this\n\
                 # harness touches it — the only moment the state the runtime\n\
                 # handed over can be read (decision 0053).\n\
                 \x20 cp \"$1\" \"$dir/../plan-seen-$$\"\n\
                 \x20 sed 's/^status: ready$/status: active/' \"$1\" > \"$1.claimed\" \\\n\
                 \x20   && mv \"$1.claimed\" \"$1\"\n\
                 \x20 {verdict}\n\
                 fi\n\
                 exit 0\n"
            ),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        rel
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

    /// Each plan as its session found it — the snapshot the fake harness
    /// takes before it touches anything, which is where what the runtime
    /// handed the session can be read (decision 0053).
    pub fn plans_as_sessions_found_them(&self) -> Vec<String> {
        let dir = self.root().join(".trellis/runtime");
        let Ok(entries) = std::fs::read_dir(dir) else {
            return Vec::new();
        };
        let mut files: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("plan-seen-"))
            .map(|e| e.path())
            .collect();
        files.sort();
        files
            .iter()
            .filter_map(|p| std::fs::read_to_string(p).ok())
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
/// answering `agent.get` from a scripted status sequence. The herdr socket is
/// config the same way the agent kind is, so a fake server is a test helper
/// the same way a fixture root is.
///
/// Statuses are scripted **per pane**, because a recycled session is a second
/// pane for the same key: one global queue would let the first attempt's
/// script leak into the retry's.
///
/// `on_prompt` is how a test plays the session itself. It fires when the
/// prompt lands, with the prompt text and the fixture root, and whatever it
/// writes into the plan is what the daemon will read back as that session's
/// verdict — which is the whole of how completion is decided (decision 0061).
#[cfg(unix)]
pub struct FakeHerdr {
    pub socket: PathBuf,
    requests: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
    peak: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

/// What a session does when its prompt arrives: `(prompt_text, root)`.
#[cfg(unix)]
pub type OnPrompt = Box<dyn Fn(&str, &Path) + Send>;

/// The ways a fake server can misbehave, which are the ways the real one has
/// been seen to. Default is a server that does everything right.
#[cfg(unix)]
#[derive(Default)]
pub struct Quirks {
    /// Refuse every `agent.start` with this code.
    pub refuse_start: Option<String>,
    /// Refuse every `agent.prompt` with this code.
    pub refuse_prompt: Option<String>,
    /// Take the first prompt and drop it: no `on_prompt`, and a pane that
    /// stays `idle` however long it is watched, until a second arrives.
    pub drop_first_prompt: bool,
}

#[cfg(unix)]
impl FakeHerdr {
    /// `statuses` feeds each pane's `agent.get`, front to back, sticking on
    /// the last; `agents` is what `agent.list` reports (adoption seeds).
    pub fn start(socket: PathBuf, statuses: &[&str], agents: Vec<serde_json::Value>) -> FakeHerdr {
        Self::start_full(
            socket,
            statuses,
            agents,
            PathBuf::new(),
            None,
            Quirks::default(),
        )
    }

    /// A server whose sessions write their own verdicts: `on_prompt` runs
    /// against `root` as each prompt lands.
    pub fn start_acting(
        socket: PathBuf,
        statuses: &[&str],
        root: &Path,
        on_prompt: OnPrompt,
    ) -> FakeHerdr {
        Self::start_full(
            socket,
            statuses,
            Vec::new(),
            root.to_path_buf(),
            Some(on_prompt),
            Quirks::default(),
        )
    }

    /// Like `start_acting`, but the hook is optional.
    pub fn start_maybe_acting(
        socket: PathBuf,
        statuses: &[&str],
        root: &Path,
        on_prompt: Option<OnPrompt>,
    ) -> FakeHerdr {
        Self::start_full(
            socket,
            statuses,
            Vec::new(),
            root.to_path_buf(),
            on_prompt,
            Quirks::default(),
        )
    }

    /// A server that refuses `agent.start` with `code` — a pane that never
    /// becomes a session.
    pub fn start_refusing(socket: PathBuf, code: &str) -> FakeHerdr {
        Self::start_full(
            socket,
            &["idle"],
            Vec::new(),
            PathBuf::new(),
            None,
            Quirks {
                refuse_start: Some(code.to_string()),
                ..Quirks::default()
            },
        )
    }

    /// A server whose agent takes the first prompt and drops it — the live
    /// seam decision 0063 confirms delivery against. Its pane stays `idle`
    /// however long it is watched until a second prompt arrives; from then
    /// on it behaves like any other, script and `on_prompt` hook included.
    pub fn start_dropping_the_first_prompt(
        socket: PathBuf,
        statuses: &[&str],
        root: &Path,
        on_prompt: OnPrompt,
    ) -> FakeHerdr {
        Self::start_full(
            socket,
            statuses,
            Vec::new(),
            root.to_path_buf(),
            Some(on_prompt),
            Quirks {
                drop_first_prompt: true,
                ..Quirks::default()
            },
        )
    }

    /// A server whose agent starts but never becomes promptable — the
    /// startup seam `agent_not_ready` names, held open forever. It reports
    /// itself unpromptable *and* refuses, which is the real pairing.
    pub fn start_never_promptable(socket: PathBuf) -> FakeHerdr {
        Self::start_full(
            socket,
            &["idle"],
            Vec::new(),
            PathBuf::new(),
            None,
            Quirks {
                refuse_prompt: Some("agent_not_ready".to_string()),
                ..Quirks::default()
            },
        )
    }

    fn start_full(
        socket: PathBuf,
        statuses: &[&str],
        agents: Vec<serde_json::Value>,
        root: PathBuf,
        on_prompt: Option<OnPrompt>,
        quirks: Quirks,
    ) -> FakeHerdr {
        let Quirks {
            refuse_start,
            refuse_prompt,
            drop_first_prompt,
        } = quirks;
        use serde_json::json;
        use std::collections::{HashMap, HashSet, VecDeque};
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixListener;

        std::fs::create_dir_all(socket.parent().unwrap()).unwrap();
        // A test may rewire its fixture to a differently-scripted server;
        // the stale socket file would otherwise refuse the bind.
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).unwrap();
        let requests = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let record = std::sync::Arc::clone(&requests);
        let script: Vec<String> = statuses.iter().map(|s| s.to_string()).collect();
        let peak = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let peak_in = std::sync::Arc::clone(&peak);

        std::thread::spawn(move || {
            let mut counter = 0u64;
            let mut seq = 10u64;
            let mut open = 0usize;
            // One status queue per pane: a retry is a new pane and starts
            // the script over.
            let mut panes: HashMap<String, VecDeque<String>> = HashMap::new();
            // Agent names herdr has handed out, which it never hands out twice.
            let mut names: HashSet<String> = HashSet::new();
            // Prompts submitted, so a server told to drop the first can.
            let mut prompts = 0u32;
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
                let refuse = |code: &str, message: &str| {
                    let mut writer = &stream;
                    let _ = writeln!(
                        writer,
                        "{}",
                        json!({"id": id, "error": {"code": code, "message": message}})
                    );
                };

                // A server that refuses every prompt reports why, the way the
                // real one does: an agent still launching is not promptable,
                // whatever its screen shows.
                let promptable = refuse_prompt.is_none();
                let agent_at = |pane: &str, status: &str, seq: u64| {
                    json!({
                        "pane_id": pane, "workspace_id": "w-fake", "tab_id": "t1",
                        "terminal_id": "term1", "focused": false, "revision": 1,
                        "agent_status": status, "state_change_seq": seq,
                        "interactive_ready": promptable,
                        "launch_pending": !promptable,
                    })
                };

                let result = match method {
                    "ping" => json!({"type": "pong", "version": "fake", "protocol": 19}),
                    "notification.show" | "pane.report_metadata" => json!({"type": "ok"}),
                    "workspace.close" => {
                        open = open.saturating_sub(1);
                        json!({"type": "ok"})
                    }
                    "workspace.create" => {
                        counter += 1;
                        open += 1;
                        peak_in.fetch_max(open, std::sync::atomic::Ordering::Relaxed);
                        json!({
                            "type": "workspace_created",
                            "workspace": {"workspace_id": format!("w{counter}")},
                            "tab": {},
                            "root_pane": {"pane_id": format!("p{counter}")},
                        })
                    }
                    "agent.start" => {
                        if let Some(code) = &refuse_start {
                            refuse(code, "fake herdr refuses to start this agent");
                            continue;
                        }
                        // The real server's other name rule: a name in use
                        // is refused, and an agent keeps its name for as
                        // long as its pane exists — a retained workspace's
                        // included. A session naming itself after its task
                        // alone made the second dispatch of a plan
                        // undispatchable (observed live, 2026-08-17).
                        if !names.insert(params["name"].as_str().unwrap_or("").to_string()) {
                            refuse(
                                "agent_name_taken",
                                "agent name is already used by another agent",
                            );
                            continue;
                        }
                        // The real server's name rule, enforced here so the
                        // hermetic tests catch what the live one refuses.
                        let name = params["name"].as_str().unwrap_or("");
                        let legal = name.len() <= 32
                            && name.starts_with(|c: char| c.is_ascii_lowercase())
                            && name.chars().all(|c| {
                                c.is_ascii_lowercase() || c.is_ascii_digit() || "-_".contains(c)
                            });
                        if !legal {
                            refuse(
                                "invalid_agent_name",
                                "agent name must start with a lowercase letter and contain only lowercase letters, digits, '-' or '_' (1-32 characters)",
                            );
                            continue;
                        }
                        json!({
                            "type": "agent_started", "argv": [],
                            "agent": agent_at(params["pane_id"].as_str().unwrap_or("p?"), "idle", 1),
                        })
                    }
                    "agent.prompt" => {
                        if let Some(code) = &refuse_prompt {
                            refuse(code, "fake herdr's agent cannot take a prompt yet");
                            continue;
                        }
                        prompts += 1;
                        // The session, such as it is: whatever the test says
                        // this prompt makes it write — unless this server
                        // drops the first one, in which case nothing about
                        // it reaches the agent at all.
                        if let Some(act) = &on_prompt {
                            if !(drop_first_prompt && prompts == 1) {
                                act(params["text"].as_str().unwrap_or(""), &root);
                            }
                        }
                        json!({
                            "type": "agent_prompted",
                            "agent": agent_at(params["target"].as_str().unwrap_or("p?"), "working", 2),
                        })
                    }
                    "agent.get" => {
                        seq += 1;
                        let pane = params["target"].as_str().unwrap_or("p?").to_string();
                        // A dropped prompt leaves the pane exactly as it
                        // was: idle, for as long as anybody cares to look.
                        if drop_first_prompt && prompts < 2 {
                            let mut writer = &stream;
                            let _ = writeln!(
                                writer,
                                "{}",
                                json!({"id": id, "result": {
                                    "type": "agent_info",
                                    "agent": agent_at(&pane, "idle", seq),
                                }})
                            );
                            continue;
                        }
                        let queue = panes
                            .entry(pane.clone())
                            .or_insert_with(|| script.iter().cloned().collect());
                        let status = if queue.len() > 1 {
                            queue.pop_front().unwrap()
                        } else {
                            queue.front().cloned().unwrap_or_else(|| "idle".into())
                        };
                        json!({
                            "type": "agent_info",
                            "agent": agent_at(&pane, &status, seq),
                        })
                    }
                    "agent.list" => json!({"type": "agent_list", "agents": agents}),
                    "agent.read" => json!({
                        "type": "pane_read",
                        "read": {"text": "fake scrollback tail\n"},
                    }),
                    other => {
                        refuse(
                            "invalid_request",
                            &format!("fake herdr does not speak {other}"),
                        );
                        continue;
                    }
                };
                let mut writer = &stream;
                let _ = writeln!(writer, "{}", json!({"id": id, "result": result}));
            }
        });
        FakeHerdr {
            socket,
            requests,
            peak,
        }
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

    /// The most workspaces open at once over the run — the concurrency the
    /// cap is supposed to bound, read from the wire rather than inferred.
    /// Only meaningful when retirement actually closes them (`retain =
    /// "never"`).
    pub fn peak_open(&self) -> usize {
        self.peak.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// How many times one method was called.
    pub fn count(&self, method: &str) -> usize {
        self.calls().iter().filter(|m| *m == method).count()
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

/// The `[harness.herdr]` block a herdr-backed fixture needs: this socket,
/// and by default no grace at all, so a settled pane is judged on the pass
/// that sees it rather than two minutes later.
#[cfg(unix)]
pub fn herdr_config(socket: &Path) -> String {
    // `prompt_deadline_secs = 0` is one submission, one look, and no waiting:
    // a fake herdr is ready the moment it answers, and a test that has to
    // wait out a real cold start is a test nobody runs.
    format!(
        "[harness.herdr]\nsocket = \"{}\"\nidle_grace_secs = 0\nprompt_deadline_secs = 0\n",
        socket.display()
    )
}

/// Every `agent.start`'s rendered flag array, in arrival order — what the
/// process backend's recorded argv used to be.
#[cfg(unix)]
impl FakeHerdr {
    pub fn started_args(&self) -> Vec<Vec<String>> {
        self.params_for("agent.start")
            .iter()
            .map(|p| {
                p["args"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|v| v.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default()
            })
            .collect()
    }

    /// Every prompt submitted, in arrival order.
    pub fn prompts(&self) -> Vec<String> {
        self.params_for("agent.prompt")
            .iter()
            .filter_map(|p| p["text"].as_str().map(str::to_string))
            .collect()
    }
}

/// The plan a dispatched prompt concerns. Every shipped prompt opens by
/// naming it, which is how a session-shaped test hook knows which artifact
/// the session it is standing in for was sent to advance.
pub fn plan_in_prompt(prompt: &str) -> Option<String> {
    prompt
        .split_whitespace()
        .next()
        .filter(|t| t.ends_with(".md"))
        .map(str::to_string)
}

/// A verdict written the way a session writes one: straight into the plan's
/// frontmatter. `status` replaces the status line; `handoff` is appended
/// when given.
pub fn write_verdict(root: &Path, rel: &str, status: &str, handoff: Option<&str>) {
    let path = root.join(rel);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let mut out = String::new();
    let mut done = false;
    for line in text.lines() {
        if !done && line.starts_with("status:") {
            out.push_str(&format!("status: {status}\n"));
            if let Some(h) = handoff {
                out.push_str(&format!("handoff: {h}\n"));
            }
            done = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    std::fs::write(&path, out).unwrap();
}

/// Wire a fixture to a fake herdr and write its `runtime.toml`: `extra` goes
/// in front (other tables), `args` is the agent flag array, and `on_prompt`
/// is the session, if the test needs one to do anything.
///
/// The grace is zero, so a settled pane is judged on the pass that sees it
/// rather than two minutes later — the tests are about the rule, not the
/// clock.
#[cfg(unix)]
pub fn wire_herdr(
    f: &Fixture,
    args: &str,
    extra: &str,
    statuses: &[&str],
    on_prompt: Option<OnPrompt>,
) -> FakeHerdr {
    wire_herdr_split(f, args, args, extra, statuses, on_prompt)
}

/// `wire_herdr` where the two errands take different flags — a ritual and an
/// act render different variables, so a template that suits one can be
/// unrenderable for the other.
#[cfg(unix)]
pub fn wire_herdr_split(
    f: &Fixture,
    act_args: &str,
    ritual_args: &str,
    extra: &str,
    statuses: &[&str],
    on_prompt: Option<OnPrompt>,
) -> FakeHerdr {
    let socket = f.root().join(".trellis/herdr.sock");
    let herdr = FakeHerdr::start_maybe_acting(socket.clone(), statuses, f.root(), on_prompt);
    f.write(
        "runtime.toml",
        &format!(
            "{extra}{}act_args = [{act_args}]\nritual_args = [{ritual_args}]\n",
            herdr_config(&socket)
        ),
    );
    herdr
}

/// A stand-in for the session itself: it snapshots what the runtime handed
/// it — the plan as it found it, and the acting-role marker while it runs —
/// and then leaves `verdict`, if it was given one.
///
/// This runs in the test process, on the fake herdr's listener thread, while
/// the daemon child is blocked in its own `agent.prompt` call. That is
/// exactly the moment a real session starts, so it is the moment at which
/// what the runtime handed over can be read (decision 0053).
#[cfg(unix)]
/// A session that declares a wait instead of a verdict — what an agent
/// minding a build it started does before it goes quiet. `written_ago` backs
/// the lease's own timestamp up, which is how a lapsed or capped one is
/// tested without sleeping.
pub fn waiting_session(root: &Path, written_ago: u64, for_secs: u64, what: &str) -> OnPrompt {
    let root = root.to_path_buf();
    let what = what.to_string();
    Box::new(move |prompt: &str, _: &Path| {
        let Some(rel) = plan_in_prompt(prompt) else {
            return;
        };
        let now = trellis::waits::now_secs().saturating_sub(written_ago);
        let _ = trellis::waits::set(&root, &rel, now, for_secs, &what);
    })
}

pub fn session(root: &Path, verdict: Option<(&str, Option<&str>)>) -> OnPrompt {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let root = root.to_path_buf();
    let verdict = verdict.map(|(s, h)| (s.to_string(), h.map(str::to_string)));
    Box::new(move |prompt: &str, _: &Path| {
        let Some(rel) = plan_in_prompt(prompt) else {
            return;
        };
        let dir = root.join(".trellis/runtime");
        let _ = std::fs::create_dir_all(&dir);
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let _ = std::fs::copy(root.join(&rel), dir.join(format!("plan-seen-{n}")));
        let _ = std::fs::copy(
            root.join(".trellis/acting-role"),
            dir.join(format!("marker-seen-{n}")),
        );
        if let Some((status, handoff)) = &verdict {
            write_verdict(&root, &rel, status, handoff.as_deref());
        }
    })
}

/// A session that uses its back-channel: it reports progress, asks a
/// question, waits for the answer, and writes what it was told to
/// `.trellis/runtime/answer`.
///
/// It reads the endpoint out of its own prompt, which is where a binding
/// that wants the session to have one puts `{mcp}`. The whole exchange is
/// plain HTTP against `/mcp/{token}`: the check that matters is the wire,
/// not who speaks it.
///
/// The waiting happens on a thread of its own. This hook runs on the fake
/// server's accept loop, and a session that blocked there would also block
/// the daemon's next `agent.get` — which is a deadlock, not a test.
#[cfg(unix)]
pub fn asking_session(question: &str) -> OnPrompt {
    let question = question.to_string();
    Box::new(move |prompt: &str, root: &Path| {
        let Some(url) = mcp_endpoint(prompt) else {
            return;
        };
        let question = question.clone();
        let root = root.to_path_buf();
        std::thread::spawn(move || {
            let post = |body: String| -> String {
                let out = Command::new("curl")
                    .args([
                        "-sS",
                        "-m",
                        "10",
                        "-H",
                        "Content-Type: application/json",
                        "-d",
                    ])
                    .arg(body)
                    .arg(&url)
                    .output()
                    .unwrap();
                String::from_utf8_lossy(&out.stdout).to_string()
            };
            post(r#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"trellis_progress","arguments":{"note":"claimed and reading"}}}"#.to_string());
            let asked = post(format!(
                r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"trellis_ask","arguments":{{"question":"{question}","options":["moves with billing","stays in core"],"context":"plans/x.md"}}}}}}"#
            ));
            let Some(ticket) = asked
                .split("ticket ")
                .nth(1)
                .and_then(|rest| rest.split_whitespace().next())
            else {
                return;
            };
            // The bounded wait, polled the way an agent willing to wait does.
            for _ in 0..30 {
                let got = post(format!(
                    r#"{{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{{"name":"trellis_await","arguments":{{"ticket":"{ticket}"}}}}}}"#
                ));
                if let Some(answer) = got
                    .split("answered: ")
                    .nth(1)
                    .and_then(|rest| rest.split('"').next())
                {
                    let dir = root.join(".trellis/runtime");
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = std::fs::write(dir.join("answer"), answer);
                    return;
                }
            }
        });
    })
}

/// The MCP endpoint a session was handed, out of a prompt that names
/// `{mcp}` — the same `--mcp-config` JSON a harness would be given.
fn mcp_endpoint(prompt: &str) -> Option<String> {
    let at = prompt.find("\"url\"")?;
    let rest = &prompt[at..];
    let open = rest.find("http")?;
    let end = rest[open..].find('"')?;
    Some(rest[open..open + end].to_string())
}

/// A session that claims its plan and walks away, leaving no verdict — the
/// failure the recycle rule exists for.
#[cfg(unix)]
pub fn abandoning_session(root: &Path) -> OnPrompt {
    session(root, None)
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
