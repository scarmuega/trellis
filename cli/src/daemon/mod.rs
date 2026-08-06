//! The local runtime: `trellis serve` (decision 0038).
//!
//! A daemon at a domain root that owns the clock — rituals on their cadence,
//! the dispatch scan on its own — spawns one headless harness session per due
//! task, and serves the tree's computed facts read-only over HTTP.
//!
//! **The boundary this module tree must not cross.** Daemon modules depend on
//! kernel modules; the kernel never depends on the daemon. The daemon
//! schedules, spawns, serves, and relays — it decides nothing. Every judgment
//! call happens inside a spawned session, under the mandate `act` resolves,
//! the gate the harness runs, and the automation class the artifact carries.
//! A rule that belongs to the model belongs in the kernel, where the lockstep
//! tests can hold it to the prose.

pub mod channels;
pub mod client;
pub mod config;
pub mod mcp;
pub mod sched;
pub mod server;
pub mod spawn;
pub mod state;
pub mod tmpl;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use self::config::RuntimeConfig;
use self::sched::Task;
use self::spawn::{InFlightView, Spawner};
use self::state::State;
use crate::dates::{self, Date};
use crate::dispatch::{self, SessionMap};
use crate::gitio::Git;
use crate::graph;
use crate::registries;
use crate::root::Root;
use crate::tree::Tree;

#[derive(Debug, Default)]
pub struct ServeOpts {
    pub root: Option<PathBuf>,
    pub plugin_root: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub bind: Option<String>,
    pub port: Option<u16>,
    pub tick_secs: Option<u64>,
    pub max_concurrent: Option<usize>,
    pub map: Vec<String>,
    /// One pass, wait for the sessions it started, exit. What a cron entry
    /// or a test invokes.
    pub once: bool,
    /// Report what would be spawned; spawn nothing, remember nothing.
    pub dry_run: bool,
    pub no_http: bool,
}

/// What `/api/status` reports: the daemon's own facts, as distinct from the
/// tree's. Everything else the server returns is computed per request from
/// the tree itself.
#[derive(Debug, Default, Serialize)]
pub struct Status {
    pub pid: u32,
    pub started: String,
    pub root: String,
    pub port: Option<u16>,
    pub dry_run: bool,
    /// The date of the last tick.
    pub last_pass: Option<String>,
    pub in_flight: Vec<InFlightView>,
    pub runs: BTreeMap<String, state::Run>,
    /// Dispatch-scan warnings from the last pass.
    pub warnings: Vec<String>,
    /// Plans held by `awaits:` at the last pass — still ready, not blocked.
    pub held: Vec<String>,
    /// `rituals.md` rows whose cadence has no day count: never auto-fired.
    pub unscheduled: Vec<String>,
}

pub struct Shared {
    pub status: Mutex<Status>,
    pub root: PathBuf,
    /// The map the dispatcher uses, so `/api/dispatch` reports the session
    /// this daemon would actually start.
    pub sessions: SessionMap,
    /// The sessions' back-channel. Read live by the server rather than through
    /// `status`, because a question raised between ticks must be answerable
    /// before the next one — a tick interval of latency is a board's budget,
    /// not an interaction's (decision 0041).
    pub inbox: mcp::Inbox,
}

/// Everything a tick needs that does not change between ticks.
struct Runtime {
    root: PathBuf,
    cfg: RuntimeConfig,
    plugin_dir: Option<String>,
    /// The port the back-channel is reachable on, once bound. `None` when the
    /// daemon is not serving, which is also when `{mcp}` is refused.
    port: Option<u16>,
    dry_run: bool,
    channels: channels::Channels,
    shared: Arc<Shared>,
}

pub fn run(opts: ServeOpts) -> anyhow::Result<ExitCode> {
    let root = open_root(opts.root.as_deref())?;
    let mut cfg = RuntimeConfig::load(&root, opts.config.as_deref())?;
    if let Some(b) = &opts.bind {
        cfg.server.bind = b.clone();
    }
    if let Some(p) = opts.port {
        cfg.server.port = p;
    }
    if let Some(t) = opts.tick_secs {
        cfg.scheduler.tick_secs = t;
    }
    if let Some(m) = opts.max_concurrent {
        cfg.scheduler.max_concurrent = m;
    }
    cfg.validate()?;

    let mut sessions = cfg.session_map()?;
    for spec in &opts.map {
        sessions.apply(spec)?;
    }

    let plugin_dir = opts
        .plugin_root
        .as_ref()
        .map(|p| p.display().to_string())
        .or_else(|| std::env::var("CLAUDE_PLUGIN_ROOT").ok());
    if plugin_dir.is_none() && names(&cfg, "{plugin_dir}") {
        anyhow::bail!(
            "the harness templates name {{plugin_dir}} but no plugin checkout is known — \
             pass --plugin-root, set CLAUDE_PLUGIN_ROOT, or drop it from {}",
            config::FILE
        );
    }

    // `--once` still does not serve. It is a cron-style pass with no operator
    // watching, so the back-channel would have nobody to answer it — and
    // binding the configured port would collide with the daemon a cron entry
    // most likely runs beside.
    let serve_http = !opts.no_http && !opts.once;
    let channel_off = !serve_http && names(&cfg, "{mcp}");
    let shared = Arc::new(Shared {
        status: Mutex::new(Status {
            pid: std::process::id(),
            started: dates::today().to_string(),
            root: root.display().to_string(),
            dry_run: opts.dry_run,
            ..Status::default()
        }),
        root: root.clone(),
        sessions,
        inbox: mcp::Inbox::default(),
    });

    let _pidfile = if opts.once {
        None
    } else {
        Some(Pidfile::claim(&root)?)
    };

    let mut rt = Runtime {
        root: root.clone(),
        cfg,
        plugin_dir,
        port: None,
        dry_run: opts.dry_run,
        // No adapter ships in this slice; a configured one is refused at
        // load. The seam is wired so adding a transport is a registration,
        // never a change to the loop.
        channels: channels::Channels::new(),
        shared: Arc::clone(&shared),
    };

    let mut _addrfile = None;
    if serve_http {
        let port = server::start(Arc::clone(&shared), &rt.cfg.server)?;
        rt.port = Some(port);
        shared.status.lock().unwrap().port = Some(port);
        // The handshake line: a caller that asked for port 0 learns which
        // port it got, and anything scripting the daemon knows the socket is
        // accepting.
        println!("listening on {}:{port}", rt.cfg.server.bind);
        // And the same fact on disk, because `trellis inbox` has to find this
        // daemon from a second terminal and `port = 0` is not discoverable
        // from the config it was configured with.
        _addrfile = Some(Addrfile::write(&root, &rt.cfg.server.bind, port)?);
    }

    let mut state = State::load(&root);
    let mut spawner = Spawner::new(&root, rt.cfg.scheduler.max_concurrent);
    if opts.once {
        spawner = spawner.once_per_task();
    }

    if !opts.once {
        signals::install();
    }

    note(&format!("serving {}", root.display()));
    if channel_off {
        // Not fatal: a session that cannot ask behaves exactly as it did
        // before the channel existed. Said out loud so the loss is not silent.
        note(&format!(
            "{} leaves no socket for a session to call back on — sessions run, \
             but cannot ask questions or report progress",
            if opts.once { "--once" } else { "--no-http" }
        ));
    }
    if rt.dry_run {
        note("dry run — nothing will be spawned and no state is written");
    }

    loop {
        let outcome = match tick(&rt, &mut state, &mut spawner) {
            Ok(outcome) => outcome,
            Err(e) => {
                note(&format!("tick failed: {e:#}"));
                Pass::default()
            }
        };
        if opts.once {
            for done in spawner.wait_all() {
                report_exit(&done);
                state.finished(&done.key, done.exit);
                retire(&rt, &done);
            }
            // A single pass starts at most `max_concurrent` sessions, so a
            // due set larger than the cap would otherwise wait for a next
            // tick this mode never has. Drain instead: keep passing while
            // work was deferred *and* the last pass made progress, which
            // never exceeds the cap at any instant and cannot spin.
            if outcome.deferred > 0 && outcome.started > 0 {
                continue;
            }
            if !rt.dry_run {
                state.save(&root)?;
            }
            return Ok(ExitCode::SUCCESS);
        }
        if sleep_until_tick_or_signal(rt.cfg.scheduler.tick_secs) {
            return shutdown(&rt, &mut state, &mut spawner);
        }
    }
}

/// Sleep out the tick interval, waking early if a stop was asked for. Returns
/// whether it was. Polled in slices rather than one long sleep: a `SIGTERM`
/// that waits a full minute to be noticed reads as a hung daemon.
fn sleep_until_tick_or_signal(tick_secs: u64) -> bool {
    let slice = std::time::Duration::from_millis(250);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(tick_secs);
    while std::time::Instant::now() < deadline {
        if signals::stopping() {
            return true;
        }
        std::thread::sleep(slice);
    }
    signals::stopping()
}

/// Stop the sessions this daemon started, then leave.
///
/// The alternative is to exit and orphan them, which is what happened before:
/// children are process-group leaders precisely so a stray Ctrl-C could not
/// kill a session mid-artifact, and the same isolation makes an intentional
/// stop something the daemon has to do on purpose.
fn shutdown(rt: &Runtime, state: &mut State, spawner: &mut Spawner) -> anyhow::Result<ExitCode> {
    let signalled = spawner.cancel_all();
    if signalled > 0 {
        note(&format!(
            "stopping — asked {signalled} session(s) to stop, waiting for them"
        ));
    }
    for done in spawner.wait_all() {
        report_exit(&done);
        state.finished(&done.key, done.exit);
        retire(rt, &done);
    }
    if !rt.dry_run {
        state.save(&rt.root)?;
    }
    note("stopped");
    Ok(ExitCode::SUCCESS)
}

/// What one pass did, so `--once` knows whether the due set is drained.
#[derive(Debug, Default)]
struct Pass {
    started: usize,
    deferred: usize,
}

/// Whether one task got a session this pass.
enum Outcome {
    Started,
    /// Already in flight, or no free slot — still due.
    Deferred,
    /// Could not be rendered or launched; reported, not retried this pass.
    Failed,
    /// Already had its session in this run; neither progress nor backlog.
    AlreadyRan,
}

fn tick(rt: &Runtime, state: &mut State, spawner: &mut Spawner) -> anyhow::Result<Pass> {
    for done in spawner.reap() {
        report_exit(&done);
        state.finished(&done.key, done.exit);
        retire(rt, &done);
    }

    let tree = Tree::load(Root {
        path: rt.root.clone(),
    })?;
    let reg = registries::load(&tree);
    let today = dates::today();
    let pass = sched::due(&reg, &rt.cfg, state, today);

    let mut warnings = Vec::new();
    let mut held = Vec::new();
    let mut outcome = Pass::default();

    for due in pass.due {
        if due.skipped {
            note(&format!(
                "{}: window missed while the daemon was down — recorded, not run (catchup = skip)",
                due.task.label()
            ));
            if !rt.dry_run {
                state.fired(&due.task.key(), today, None);
            }
            continue;
        }
        match &due.task {
            Task::Ritual { name, executor } => {
                let vars = tmpl::Vars::new().set("ritual", name.clone());
                outcome.record(fire(
                    rt,
                    state,
                    spawner,
                    &due.task.key(),
                    &format!("ritual {name} ({executor})"),
                    &rt.cfg.prompts.ritual,
                    &rt.cfg.harness.ritual_cmd,
                    vars,
                    today,
                ));
            }
            Task::DispatchScan => {
                let report = dispatch::scan(&tree, &rt.shared.sessions);
                for w in &report.warnings {
                    note(&format!("dispatch: {w}"));
                }
                for h in &report.held {
                    note(&format!(
                        "holding {} — awaits {} (status: {}); it stays ready and re-enters the scan next tick",
                        h.plan,
                        h.awaits,
                        h.target_status.as_deref().unwrap_or("missing")
                    ));
                    held.push(h.plan.clone());
                }
                warnings = report.warnings.clone();

                let mut all_started = true;
                for item in &report.dispatch {
                    let vars = tmpl::Vars::new()
                        .set("plan", item.plan.clone())
                        .set("owner", item.owner_short.clone())
                        .set("complexity", item.complexity.clone())
                        .set("model", item.model.clone())
                        .set("effort", item.effort.clone())
                        .set("budget", item.budget_usd.to_string());
                    let started = fire(
                        rt,
                        state,
                        spawner,
                        &state::key_plan(&item.plan),
                        &format!("act {} → {}", item.plan, item.owner),
                        &rt.cfg.prompts.act,
                        &rt.cfg.harness.act_cmd,
                        vars,
                        today,
                    );
                    all_started &= !matches!(started, Outcome::Deferred);
                    outcome.record(started);
                }
                // The cadence says how often to scan, never how many plans
                // may start in a day. So a scan that could not start
                // everything it found is not recorded as done: it stays due
                // and re-runs as soon as a slot frees, rather than leaving
                // the overflow to wait a full cadence.
                if !rt.dry_run && all_started {
                    state.fired(state::DISPATCH, today, None);
                }
            }
        }
    }

    if !rt.dry_run {
        // With no adapter registered, announcing is a no-op and the log line
        // below is the whole channel: the records are in the tree, and
        // /api/escalations and the board read them there.
        let fresh = channels::detect_new(&tree, state);
        for record in &fresh {
            note(&format!(
                "escalation opened: {} → {} ({})",
                record.artifact,
                record.to.as_deref().unwrap_or("?"),
                record.asks.as_deref().unwrap_or("")
            ));
        }
        for failure in rt.channels.announce(&fresh) {
            note(&failure);
        }
    }

    {
        let mut status = rt.shared.status.lock().unwrap();
        status.last_pass = Some(today.to_string());
        status.in_flight = spawner.in_flight();
        status.runs = state.runs.clone();
        status.warnings = warnings;
        status.held = held;
        status.unscheduled = pass.unscheduled;
    }

    if !rt.dry_run {
        state.save(&rt.root)?;
    }
    Ok(outcome)
}

impl Pass {
    fn record(&mut self, outcome: Outcome) {
        match outcome {
            Outcome::Started => self.started += 1,
            Outcome::Deferred => self.deferred += 1,
            Outcome::Failed | Outcome::AlreadyRan => {}
        }
    }
}

/// Render one session's command and start it — unless the task is already in
/// flight or every slot is taken, in which case it stays due and starts on a
/// later pass.
#[allow(clippy::too_many_arguments)]
fn fire(
    rt: &Runtime,
    state: &mut State,
    spawner: &mut Spawner,
    key: &str,
    label: &str,
    prompt_template: &str,
    argv_template: &[String],
    vars: tmpl::Vars,
    today: Date,
) -> Outcome {
    if spawner.is_busy(key) {
        note(&format!(
            "{label}: already in flight — not starting a second"
        ));
        return Outcome::Deferred;
    }
    if spawner.already_ran(key) {
        return Outcome::AlreadyRan;
    }
    if spawner.capacity() == 0 {
        note(&format!(
            "{label}: all {} session slots busy — deferred, still due",
            rt.cfg.scheduler.max_concurrent
        ));
        return Outcome::Deferred;
    }

    let mut vars = vars.set("root", rt.root.display().to_string());
    if let Some(dir) = &rt.plugin_dir {
        vars = vars.set("plugin_dir", dir.clone());
    }

    // The channel is opened before the argv is rendered, because `{mcp}`
    // carries its handle. Anything that fails from here on closes it again:
    // a channel outliving the session it belongs to would be a question
    // nobody is left to answer.
    let mut token = None;
    match rt.port {
        Some(port) => {
            let handle = rt.shared.inbox.register(key, label);
            vars = vars.set("mcp", mcp::config_json(&rt.cfg.server.bind, port, &handle));
            token = Some(handle);
        }
        // No listener, so no channel — but the template still has to render.
        None => vars = vars.set("mcp", mcp::disabled_json()),
    }
    let close = || {
        if let Some(token) = &token {
            rt.shared.inbox.retire(token);
        }
    };

    let argv = match render(prompt_template, argv_template, &vars) {
        Ok(argv) => argv,
        Err(e) => {
            note(&format!("{label}: {e:#}"));
            close();
            return Outcome::Failed;
        }
    };

    if rt.dry_run {
        note(&format!("would run {label}: {}", spawn::shell_quote(&argv)));
        close();
        return Outcome::Started;
    }
    match spawner.spawn(key, label, &argv, token.clone()) {
        Ok(log) => {
            note(&format!("started {label} → logs/{log}"));
            state.fired(key, today, Some(log));
            Outcome::Started
        }
        Err(e) => {
            note(&format!("{label}: {e:#}"));
            close();
            Outcome::Failed
        }
    }
}

/// Substitute the prompt first, then the command that carries it — so a
/// prompt is one argument no matter what it contains.
fn render(
    prompt_template: &str,
    argv_template: &[String],
    vars: &tmpl::Vars,
) -> anyhow::Result<Vec<String>> {
    let prompt = tmpl::substitute("prompt", &[prompt_template.to_string()], vars)?
        .into_iter()
        .next()
        .expect("one element in, one out");
    tmpl::substitute(
        "command",
        argv_template,
        &vars.clone().set("prompt", prompt),
    )
}

/// Close a finished session's back-channel. A question outstanding when its
/// session exits goes with it: there is nobody left to receive the answer, and
/// leaving it listed would invite an operator to answer into a void.
fn retire(rt: &Runtime, done: &spawn::Finished) {
    if let Some(token) = &done.token {
        rt.shared.inbox.retire(token);
    }
}

fn report_exit(done: &spawn::Finished) {
    match done.exit {
        Some(0) => note(&format!("finished {}", done.label)),
        Some(code) => note(&format!("finished {} — exit {code}", done.label)),
        None => note(&format!("finished {} — killed by signal", done.label)),
    }
}

/// The daemon's own log line. Stdout is the supervisor's to capture; session
/// output goes to its own file.
pub fn note(message: &str) {
    println!("[trellis] {message}");
}

/// Whether any template names a placeholder — asked of the ones the daemon
/// must be able to *supply* before the first tick, so a template that names
/// something unavailable is refused at startup rather than at the first spawn.
fn names(cfg: &RuntimeConfig, placeholder: &str) -> bool {
    let uses = |t: &[String]| t.iter().any(|e| e.contains(placeholder));
    uses(&cfg.harness.act_cmd)
        || uses(&cfg.harness.ritual_cmd)
        || cfg.prompts.act.contains(placeholder)
        || cfg.prompts.ritual.contains(placeholder)
}

fn open_root(explicit: Option<&Path>) -> anyhow::Result<PathBuf> {
    let root = Root::discover(explicit)?;
    if root.is_plugin_template() {
        anyhow::bail!(
            "{} is the plugin's own template, not a domain root",
            root.path.display()
        );
    }
    Ok(root.path)
}

/// Two daemons on one root would double-spawn every session. The HTTP port
/// already collides in the common case; this covers `--no-http`.
struct Pidfile(PathBuf);

impl Pidfile {
    fn claim(root: &Path) -> anyhow::Result<Pidfile> {
        let dir = RuntimeConfig::runtime_dir(root);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join("serve.pid");
        if let Ok(text) = std::fs::read_to_string(&path) {
            let pid = text.trim().to_string();
            if alive(&pid) {
                anyhow::bail!(
                    "another trellis serve is already running on this root (pid {pid}); \
                     stop it, or remove {} if it is stale",
                    path.display()
                );
            }
        }
        std::fs::write(&path, format!("{}\n", std::process::id()))?;
        Ok(Pidfile(path))
    }
}

impl Drop for Pidfile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// Where this daemon is listening, for the commands that talk to it.
pub const ADDRFILE: &str = "serve.addr";

struct Addrfile(PathBuf);

impl Addrfile {
    fn write(root: &Path, bind: &str, port: u16) -> anyhow::Result<Addrfile> {
        let dir = RuntimeConfig::runtime_dir(root);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(ADDRFILE);
        // A wildcard bind is dialled back on loopback, the same substitution
        // the sessions' own endpoint makes.
        let host = match bind {
            "0.0.0.0" | "::" | "[::]" | "" => "127.0.0.1",
            other => other,
        };
        std::fs::write(&path, format!("{host}:{port}\n"))?;
        Ok(Addrfile(path))
    }
}

impl Drop for Addrfile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

#[cfg(unix)]
fn alive(pid: &str) -> bool {
    pid.parse::<u32>().is_ok()
        && std::process::Command::new("kill")
            .args(["-0", pid])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn alive(_pid: &str) -> bool {
    false
}

/// Stopping on request rather than dying where we stand.
///
/// `SIGTERM` only, deliberately. Children are process-group leaders so that a
/// Ctrl-C to the daemon does not kill a session mid-artifact (`spawn`), and
/// that stays true: `SIGINT` keeps its default, and stopping the sessions too
/// is the deliberate act `kill` or a service manager performs.
#[cfg(unix)]
mod signals {
    use std::sync::atomic::{AtomicBool, Ordering};

    static STOPPING: AtomicBool = AtomicBool::new(false);

    extern "C" fn on_term(_sig: libc::c_int) {
        // Setting a flag is the only async-signal-safe thing done here. The
        // work it implies happens on the main loop's own time.
        STOPPING.store(true, Ordering::SeqCst);
    }

    pub fn install() {
        let handler = on_term as extern "C" fn(libc::c_int);
        // SAFETY: installs a handler that only stores to an atomic.
        unsafe {
            libc::signal(libc::SIGTERM, handler as libc::sighandler_t);
        }
    }

    pub fn stopping() -> bool {
        STOPPING.load(Ordering::SeqCst)
    }
}

#[cfg(not(unix))]
mod signals {
    pub fn install() {}
    pub fn stopping() -> bool {
        false
    }
}

/// A fresh view of the tree, its derived graph, and git — what every HTTP
/// handler computes from. The scale this serves is one repository, and a
/// stale answer is worse than a slow one, so freshness beats caching until it
/// measurably does not.
pub fn snapshot(root: &Path) -> anyhow::Result<(Tree, Git, graph::Derived, Date)> {
    let root = Root {
        path: root.to_path_buf(),
    };
    let git = Git::new(root.path.clone());
    let tree = Tree::load(root)?;
    let derived = graph::derive(&tree);
    Ok((tree, git, derived, dates::today()))
}
