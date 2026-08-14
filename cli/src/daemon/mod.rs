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

pub mod backend;
pub mod channels;
pub mod client;
pub mod config;
pub mod marker;
#[cfg(unix)]
pub mod herdr;
pub mod mcp;
pub mod procedure;
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

use self::backend::{Backend, Launch};
use self::config::RuntimeConfig;
use self::spawn::InFlightView;
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
    pub config: Option<PathBuf>,
    pub bind: Option<String>,
    pub port: Option<u16>,
}

#[derive(Debug, Default)]
pub struct DispatchOpts {
    pub root: Option<PathBuf>,
    pub plugin_root: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub tick_secs: Option<u64>,
    pub max_concurrent: Option<usize>,
    pub map: Vec<String>,
    /// One pass, wait for the sessions it started, exit.
    pub once: bool,
    /// Report what would be spawned; spawn nothing, remember nothing.
    pub dry_run: bool,
}

#[derive(Debug, Default)]
pub struct RitualsOpts {
    pub root: Option<PathBuf>,
    pub plugin_root: Option<PathBuf>,
    pub config: Option<PathBuf>,
    pub max_concurrent: Option<usize>,
    pub dry_run: bool,
}

/// The dispatcher's own facts, snapshotted to `status.json` every pass so the
/// read-only server can show them without sharing a process (decision 0046).
#[derive(Debug, Default, Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Status {
    pub pid: u32,
    /// Which process wrote this: "serve" | "dispatch" | "rituals".
    pub process: String,
    pub started: String,
    pub root: String,
    pub port: Option<u16>,
    pub dry_run: bool,
    /// The date of the last pass.
    pub last_pass: Option<String>,
    pub in_flight: Vec<InFlightView>,
    pub runs: BTreeMap<String, state::Run>,
    /// Dispatch-scan warnings from the last pass.
    pub warnings: Vec<String>,
    /// Plans held at the last pass — still ready, not blocked.
    pub held: Vec<String>,
    /// Ready plans whose owner is human-held: no session, a handoff.
    pub handoffs: Vec<String>,
    /// `rituals.md` rows whose cadence has no day count: never auto-fired.
    pub unscheduled: Vec<String>,
}

/// The dispatcher's status snapshot on disk, for the read-only server.
pub const STATUS_FILE: &str = "status.json";

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
    /// The read-only server overlays the dispatcher's `status.json` — it has
    /// no pass of its own to report (decision 0046).
    pub overlay_status: bool,
    /// This process runs the dispatch loop, so its socket honors errand
    /// requests locally (decisions 0048, 0051); everywhere else the route
    /// relays or refuses.
    pub dispatches: bool,
    /// Errand sessions an operator asked for, drained by the next pass. The
    /// queue is memory only — a request is one-shot and best-effort, and the
    /// loop stays the only spawner.
    pub errands: Mutex<Vec<ErrandRequest>>,
    /// The operator-requestable errand names this config carries: every
    /// `[prompts]` key except `act` and `ritual`, which belong to their
    /// triggers (decision 0051). Sorted, computed once at startup.
    pub errands_available: Vec<String>,
    /// Ask the loop to pass now without queueing anything. Set when a request
    /// is refused as in-flight: `status.in_flight` is only as fresh as the
    /// last pass, so if that session actually ended, the nudged pass reaps it
    /// and the operator's retry succeeds within a second instead of a tick.
    pub nudge: std::sync::atomic::AtomicBool,
}

/// The `[prompts]` keys reserved for the runtime's own triggers: the dispatch
/// loop and the cadence pass. Everything else in the map is an operator's to
/// request.
pub const TRIGGER_ERRANDS: &[&str] = &["act", "ritual"];

/// The requestable errand names a config carries, sorted.
pub fn errands_available(cfg: &RuntimeConfig) -> Vec<String> {
    let mut names: Vec<String> = cfg
        .prompts
        .keys()
        .filter(|k| !TRIGGER_ERRANDS.contains(&k.as_str()))
        .cloned()
        .collect();
    names.sort();
    names
}

/// One operator ask: run this errand over this plan, per this instruction.
#[derive(Debug, Clone)]
pub struct ErrandRequest {
    /// A `[prompts]` key that is not a trigger's ("refine", or whatever the
    /// instance added).
    pub errand: String,
    /// The plan rel ("plans/x.md").
    pub plan: String,
    pub instruction: String,
}

impl Shared {
    /// Queue an errand. A second request for a plan already queued replaces
    /// its errand and instruction — the operator changed their mind, not
    /// their target, and one plan carries one session either way.
    pub fn request_errand(&self, errand: &str, plan: &str, instruction: &str) {
        let mut queue = self.errands.lock().unwrap();
        if let Some(existing) = queue.iter_mut().find(|r| r.plan == plan) {
            existing.errand = errand.to_string();
            existing.instruction = instruction.to_string();
        } else {
            queue.push(ErrandRequest {
                errand: errand.to_string(),
                plan: plan.to_string(),
                instruction: instruction.to_string(),
            });
        }
    }

    pub fn take_errands(&self) -> Vec<ErrandRequest> {
        std::mem::take(&mut *self.errands.lock().unwrap())
    }

    pub fn has_errands(&self) -> bool {
        !self.errands.lock().unwrap().is_empty()
    }
}

/// Why an errand request is refused, in the tree's own vocabulary. Checked
/// at the route when the request arrives, and again by the pass that drains
/// it — the tree may have changed in between, and the loop trusts only its
/// own snapshot (decision 0048).
#[derive(Debug)]
pub enum ErrandRefusal {
    /// Not a plan in this root, live or archived.
    NotAPlan,
    /// Retired or archived: the terminal tier is frozen content.
    Terminal,
    /// No `owner:` — no mandate to act under.
    Unowned,
    /// The owner's holder is a declared human: the work is theirs to do by
    /// hand, not a session's to run.
    Handoff {
        owner: String,
        holder_ref: Option<String>,
    },
}

impl ErrandRefusal {
    pub fn describe(&self, rel: &str) -> String {
        match self {
            ErrandRefusal::NotAPlan => format!("{rel} is not a plan in this root"),
            ErrandRefusal::Terminal => {
                format!("{rel} is in the terminal tier — errands work live plans, not history")
            }
            ErrandRefusal::Unowned => {
                format!("{rel} declares no owner: — there is no mandate to act under")
            }
            ErrandRefusal::Handoff { owner, holder_ref } => format!(
                "{rel}: {owner} is human-held{} — this is a handoff, not a session",
                holder_ref
                    .as_deref()
                    .map(|r| format!(" ({r})"))
                    .unwrap_or_default()
            ),
        }
    }
}

/// Whether one plan may carry an errand session right now; `Ok` carries its
/// `owner:`.
pub fn validate_errand(tree: &Tree, rel: &str) -> Result<String, ErrandRefusal> {
    let Some(plan) = tree.get_addressed(rel) else {
        return Err(ErrandRefusal::NotAPlan);
    };
    if plan.kind != crate::tree::Kind::Plan {
        return Err(ErrandRefusal::NotAPlan);
    }
    if plan.archived || plan.status().as_deref() == Some("retired") {
        return Err(ErrandRefusal::Terminal);
    }
    let Some(owner) = plan.owner() else {
        return Err(ErrandRefusal::Unowned);
    };
    let owner_short = owner.strip_prefix("org/").unwrap_or(&owner).to_string();
    let holder = crate::org::holder(tree, &owner_short);
    if holder.is_declared_human() {
        return Err(ErrandRefusal::Handoff {
            owner,
            holder_ref: holder.reference,
        });
    }
    Ok(owner)
}

/// Everything a pass needs that does not change between passes.
struct Runtime {
    root: PathBuf,
    cfg: RuntimeConfig,
    plugin_dir: Option<String>,
    /// The act body every prompt's `{procedure}` renders (decisions 0050,
    /// 0051) — the embedded copy, or the plugin checkout's when one is known.
    procedure: String,
    /// The port the back-channel is reachable on, once bound. `None` under
    /// `--dry-run`, which spawns nothing that could call back.
    port: Option<u16>,
    dry_run: bool,
    channels: channels::Channels,
    /// Tickets already pushed to the channels. In memory only: tickets die
    /// with the daemon, so there is nothing to remember across restarts.
    announced_tickets: Mutex<std::collections::HashSet<String>>,
    shared: Arc<Shared>,
}

/// What the dispatcher has already said, so a tight loop reports transitions
/// instead of repeating every standing fact once per tick.
#[derive(Default)]
struct Reported {
    held: std::collections::HashSet<String>,
    handoffs: std::collections::HashSet<String>,
    warnings: std::collections::HashSet<String>,
    cooling: std::collections::HashSet<String>,
}

/// `trellis serve` — the read-only surface, and nothing else (decision
/// 0046): the tree, the views, the board, and the dispatcher's snapshot.
/// No clock, no spawning, no writes; safe to kill, safe to not run.
pub fn run_serve(opts: ServeOpts) -> anyhow::Result<ExitCode> {
    let root = open_root(opts.root.as_deref())?;
    let mut cfg = RuntimeConfig::load(&root, opts.config.as_deref())?;
    if let Some(b) = &opts.bind {
        cfg.server.bind = b.clone();
    }
    if let Some(p) = opts.port {
        cfg.server.port = p;
    }
    // No full validate(): serve spawns nothing and announces nothing, so a
    // harness or channel problem is the dispatcher's to refuse, not the
    // viewer's.
    let sessions = cfg.session_map()?;
    let shared = Arc::new(Shared {
        status: Mutex::new(Status {
            pid: std::process::id(),
            process: "serve".into(),
            started: dates::today().to_string(),
            root: root.display().to_string(),
            ..Status::default()
        }),
        root: root.clone(),
        sessions,
        inbox: mcp::Inbox::default(),
        overlay_status: true,
        dispatches: false,
        errands: Mutex::new(Vec::new()),
        errands_available: errands_available(&cfg),
        nudge: std::sync::atomic::AtomicBool::new(false),
    });
    let _pidfile = Pidfile::claim(&root, "serve.pid", "trellis serve")?;
    let port = server::start(Arc::clone(&shared), &cfg.server)?;
    shared.status.lock().unwrap().port = Some(port);
    println!("listening on {}:{port}", cfg.server.bind);
    let _addrfile = Addrfile::write(&root, SERVE_ADDRFILE, &cfg.server.bind, port)?;
    note(&format!(
        "serving {} read-only — dispatch and rituals are their own processes (decision 0046)",
        root.display()
    ));
    signals::install();
    loop {
        if signals::stopping() {
            note("stopped");
            return Ok(ExitCode::SUCCESS);
        }
        std::thread::sleep(std::time::Duration::from_millis(250));
    }
}

/// `trellis dispatch run` — the pull loop (decision 0046): poll the tree
/// every tick, spawn an act for everything dispatchable, no calendar gate.
/// Idempotency is the taker's claim; the only throttle a tight loop needs is
/// the retry cooldown, or a session that dies unclaiming respawns per tick.
pub fn run_dispatch(opts: DispatchOpts) -> anyhow::Result<ExitCode> {
    let root = open_root(opts.root.as_deref())?;
    let mut cfg = RuntimeConfig::load(&root, opts.config.as_deref())?;
    if let Some(t) = opts.tick_secs {
        cfg.scheduler.tick_secs = t;
    }
    if let Some(m) = opts.max_concurrent {
        cfg.scheduler.max_concurrent = m;
    }
    cfg.validate()?;
    if cfg.scheduler.dispatch_cadence.is_some() {
        note(
            "scheduler.dispatch_cadence is obsolete — dispatch polls every tick \
             (decision 0046); drop it from runtime.toml",
        );
    }
    let mut sessions = cfg.session_map()?;
    for spec in &opts.map {
        sessions.apply(spec)?;
    }
    let plugin_dir = resolve_plugin_dir(&opts.plugin_root, &cfg)?;
    let procedure = procedure::load(plugin_dir.as_deref().map(Path::new))?;

    let shared = Arc::new(Shared {
        status: Mutex::new(Status {
            pid: std::process::id(),
            process: "dispatch".into(),
            started: dates::today().to_string(),
            root: root.display().to_string(),
            dry_run: opts.dry_run,
            ..Status::default()
        }),
        root: root.clone(),
        sessions,
        inbox: mcp::Inbox::default(),
        overlay_status: false,
        dispatches: true,
        errands: Mutex::new(Vec::new()),
        errands_available: errands_available(&cfg),
        nudge: std::sync::atomic::AtomicBool::new(false),
    });
    let _pidfile = if opts.once {
        None
    } else {
        Some(Pidfile::claim(&root, "dispatch.pid", "trellis dispatch")?)
    };
    let channels = open_channels(&cfg)?;
    let mut rt = Runtime {
        root: root.clone(),
        cfg,
        plugin_dir,
        procedure,
        port: None,
        dry_run: opts.dry_run,
        channels,
        announced_tickets: Mutex::new(std::collections::HashSet::new()),
        shared: Arc::clone(&shared),
    };

    // The back-channel binds loopback on an OS-assigned port, `--once`
    // included: a dispatcher always has somebody to answer it — the
    // operator's inbox. Only a dry run, which spawns nothing that could
    // call back, goes without.
    let mut _addrfile = None;
    if !rt.dry_run {
        let channel = config::Server {
            bind: "127.0.0.1".into(),
            port: 0,
        };
        let port = server::start(Arc::clone(&shared), &channel)?;
        rt.port = Some(port);
        shared.status.lock().unwrap().port = Some(port);
        // The handshake line: scripts learn the channel port the same way
        // serve announces its listener.
        println!("channel on 127.0.0.1:{port}");
        _addrfile = Some(Addrfile::write(&root, DISPATCH_ADDRFILE, "127.0.0.1", port)?);
    }

    let mut state = State::load_dispatch(&root);
    let mut backend = Backend::connect(&root, &rt.cfg, opts.once, "plan:")?;

    // A crash removed no marker lines and adopted sessions should keep
    // theirs: reconcile `.trellis/acting-role` against what is actually in
    // flight before the first pass can spawn anything.
    if !rt.dry_run {
        let live: Vec<String> = backend.in_flight().iter().map(|s| s.key.clone()).collect();
        if let Err(e) = marker::reconcile(&root, "plan:", &live) {
            note(&format!("acting-role marker reconcile failed: {e}"));
        }
    }

    if !opts.once {
        signals::install();
    }
    note(&format!(
        "dispatching {} — every {}s, no calendar gate (decision 0046)",
        root.display(),
        rt.cfg.scheduler.tick_secs
    ));
    if rt.dry_run {
        note("dry run — nothing will be spawned and no state is written");
    }

    let mut reported = Reported::default();
    loop {
        let outcome = match dispatch_pass(&rt, &mut state, &mut backend, &mut reported) {
            Ok(outcome) => outcome,
            Err(e) => {
                note(&format!("pass failed: {e:#}"));
                Pass::default()
            }
        };
        if opts.once {
            for done in backend.drain() {
                report_exit(&done);
                state.finished(&done.key, done.exit);
                retire(&rt, &state, &done);
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
        if sleep_until_tick_or_signal(rt.cfg.scheduler.tick_secs, &shared) {
            return shutdown(&rt, &mut state, &mut backend);
        }
    }
}

/// `trellis rituals` — one pass of the wall-clock work (decision 0046):
/// fire what the cadences owe today, wait it out, record, exit. Idempotent
/// per day, so the OS may invoke it as often as it likes — cron and launchd
/// are better wall clocks than a sleeping daemon ever was.
pub fn run_rituals(opts: RitualsOpts) -> anyhow::Result<ExitCode> {
    let root = open_root(opts.root.as_deref())?;
    let mut cfg = RuntimeConfig::load(&root, opts.config.as_deref())?;
    if let Some(m) = opts.max_concurrent {
        cfg.scheduler.max_concurrent = m;
    }
    cfg.validate()?;
    let sessions = cfg.session_map()?;
    let plugin_dir = resolve_plugin_dir(&opts.plugin_root, &cfg)?;
    let procedure = procedure::load(plugin_dir.as_deref().map(Path::new))?;

    let shared = Arc::new(Shared {
        status: Mutex::new(Status {
            pid: std::process::id(),
            process: "rituals".into(),
            started: dates::today().to_string(),
            root: root.display().to_string(),
            dry_run: opts.dry_run,
            ..Status::default()
        }),
        root: root.clone(),
        sessions,
        inbox: mcp::Inbox::default(),
        overlay_status: false,
        dispatches: false,
        errands: Mutex::new(Vec::new()),
        errands_available: errands_available(&cfg),
        nudge: std::sync::atomic::AtomicBool::new(false),
    });
    // No channels: announcing newly-open records is the dispatcher's watch.
    let mut rt = Runtime {
        root: root.clone(),
        cfg,
        plugin_dir,
        procedure,
        port: None,
        dry_run: opts.dry_run,
        channels: channels::Channels::new(),
        announced_tickets: Mutex::new(std::collections::HashSet::new()),
        shared: Arc::clone(&shared),
    };

    let mut _addrfile = None;
    if !rt.dry_run {
        let channel = config::Server {
            bind: "127.0.0.1".into(),
            port: 0,
        };
        let port = server::start(Arc::clone(&shared), &channel)?;
        rt.port = Some(port);
        _addrfile = Some(Addrfile::write(&root, RITUALS_ADDRFILE, "127.0.0.1", port)?);
    }

    let mut state = State::load_rituals(&root);
    let mut backend = Backend::connect(&root, &rt.cfg, true, "ritual:")?;
    if !rt.dry_run {
        let live: Vec<String> = backend.in_flight().iter().map(|s| s.key.clone()).collect();
        if let Err(e) = marker::reconcile(&root, "ritual:", &live) {
            note(&format!("acting-role marker reconcile failed: {e}"));
        }
    }
    note(&format!("rituals pass for {}", root.display()));
    if rt.dry_run {
        note("dry run — nothing will be spawned and no state is written");
    }

    let mut first = true;
    loop {
        let outcome = rituals_pass(&rt, &mut state, &mut backend, first)?;
        first = false;
        for done in backend.drain() {
            report_exit(&done);
            state.finished(&done.key, done.exit);
            retire(&rt, &state, &done);
        }
        if outcome.deferred > 0 && outcome.started > 0 {
            continue;
        }
        if !rt.dry_run {
            state.save(&root)?;
        }
        return Ok(ExitCode::SUCCESS);
    }
}

/// `--plugin-root`, `$CLAUDE_PLUGIN_ROOT`, or a refusal when a template
/// names `{plugin_dir}` and neither is set.
fn resolve_plugin_dir(
    explicit: &Option<PathBuf>,
    cfg: &RuntimeConfig,
) -> anyhow::Result<Option<String>> {
    let plugin_dir = explicit
        .as_ref()
        .map(|p| p.display().to_string())
        .or_else(|| std::env::var("CLAUDE_PLUGIN_ROOT").ok());
    if plugin_dir.is_none() && names(cfg, "{plugin_dir}") {
        anyhow::bail!(
            "the harness templates name {{plugin_dir}} but no plugin checkout is known — \
             pass --plugin-root, set CLAUDE_PLUGIN_ROOT, or drop it from {}",
            config::FILE
        );
    }
    Ok(plugin_dir)
}

/// The escalation transports this run announces on, built from config. The
/// only kind that reaches here is "herdr" — `validate` refused the rest —
/// and one that is configured but unreachable is refused outright, on the
/// `{plugin_dir}` precedent: configured-and-dead is wrong, not degraded.
fn open_channels(cfg: &RuntimeConfig) -> anyhow::Result<channels::Channels> {
    let mut channels = channels::Channels::new();
    for entry in &cfg.channels {
        #[cfg(unix)]
        {
            let client = herdr::client::Client::new(entry.socket.as_deref());
            client
                .handshake()
                .map_err(|e| anyhow::anyhow!("channels: {e:#} — or drop the [[channels]] entry"))?;
            note(&format!(
                "channel herdr: escalations will toast at {}",
                client.socket().display()
            ));
            channels.register(Box::new(herdr::channel::HerdrChannel::new(client)));
        }
        #[cfg(not(unix))]
        {
            let _ = entry;
            anyhow::bail!("channel kind \"herdr\" speaks a unix socket — this platform has none");
        }
    }
    Ok(channels)
}

/// Sleep out the tick interval, waking early if a stop was asked for. Returns
/// whether it was. Polled in slices rather than one long sleep: a `SIGTERM`
/// that waits a full minute to be noticed reads as a hung daemon. The same
/// slices notice a queued errand request (decision 0048), so an operator's
/// ask starts within a fraction of a second, not a tick — no Condvar needed
/// on a loop that already polls this often.
fn sleep_until_tick_or_signal(tick_secs: u64, shared: &Shared) -> bool {
    let slice = std::time::Duration::from_millis(250);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(tick_secs);
    while std::time::Instant::now() < deadline {
        if signals::stopping() {
            return true;
        }
        if shared.has_errands() {
            return false;
        }
        if shared
            .nudge
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            return false;
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
/// stop something the daemon has to do on purpose. The herdr backend may be
/// configured to detach instead — its sessions live in herdr, not under this
/// process, and the next daemon adopts them.
fn shutdown(rt: &Runtime, state: &mut State, backend: &mut Backend) -> anyhow::Result<ExitCode> {
    let (signalled, done) = backend.stop_all();
    if signalled > 0 {
        note(&format!(
            "stopping — asked {signalled} session(s) to stop, waiting for them"
        ));
    }
    for done in done {
        report_exit(&done);
        state.finished(&done.key, done.exit);
        retire(rt, state, &done);
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

/// One dispatcher pass: reap, scan, fire, announce, snapshot. No calendar —
/// the scan is the cheap deterministic read the spec always said it was, and
/// running it every tick is what lets an `awaits:` release dispatch within a
/// minute instead of a day (decision 0046).
fn dispatch_pass(
    rt: &Runtime,
    state: &mut State,
    backend: &mut Backend,
    reported: &mut Reported,
) -> anyhow::Result<Pass> {
    for done in backend.reap() {
        report_exit(&done);
        state.finished(&done.key, done.exit);
        retire(rt, state, &done);
    }

    let tree = Tree::load(Root {
        path: rt.root.clone(),
    })?;
    let today = dates::today();
    let derived = graph::derive(&tree);
    let report = dispatch::scan(&tree, &derived, &rt.shared.sessions);

    // Transitions, not standing facts: a tight loop that repeated every hold
    // once per tick would bury its own log.
    for w in &report.warnings {
        if reported.warnings.insert(w.clone()) {
            note(&format!("dispatch: {w}"));
        }
    }
    let held_now: std::collections::HashSet<String> = report
        .held
        .iter()
        .map(|h| format!("{} — {}", h.plan, h.describe()))
        .collect();
    for h in &report.held {
        let line = format!("{} — {}", h.plan, h.describe());
        if reported.held.insert(line.clone()) {
            note(&format!("holding {line}; it stays ready and re-enters the scan"));
        }
    }
    for gone in reported.held.difference(&held_now).cloned().collect::<Vec<_>>() {
        reported.held.remove(&gone);
        note(&format!("hold cleared: {gone}"));
    }
    for h in &report.handoffs {
        if reported.handoffs.insert(h.plan.clone()) {
            note(&format!(
                "handoff: {} → {} is human-held{} — no session; the work awaits its holder",
                h.plan,
                h.owner,
                h.holder_ref
                    .as_deref()
                    .map(|r| format!(" ({r})"))
                    .unwrap_or_default()
            ));
        }
    }
    reported
        .handoffs
        .retain(|p| report.handoffs.iter().any(|h| &h.plan == p));

    let mut outcome = Pass::default();

    // Operator-requested errand sessions drain first (decisions 0048, 0051):
    // an explicit ask outranks the scheduled scan for the same slots. Each is
    // one-shot — re-validated against this pass's tree, then fired or dropped
    // with its reason, never carried to a later pass. No cooldown: the
    // cooldown throttles unattended respawns, and this is the attended case.
    for req in rt.shared.take_errands() {
        let Some(prompt) = rt.cfg.prompts.get(&req.errand) else {
            note(&format!(
                "{} {} dropped — no [prompts] entry names that errand",
                req.errand, req.plan
            ));
            continue;
        };
        let owner = match validate_errand(&tree, &req.plan) {
            Ok(owner) => owner,
            Err(refusal) => {
                note(&format!(
                    "{} dropped — {}",
                    req.errand,
                    refusal.describe(&req.plan)
                ));
                continue;
            }
        };
        let owner_short = owner.strip_prefix("org/").unwrap_or(&owner).to_string();
        let (complexity, session) =
            dispatch::plan_session(&tree, &rt.shared.sessions, &req.plan);
        let key = state::key_plan(&req.plan);
        let vars = tmpl::Vars::new()
            .set("plan", req.plan.clone())
            .set("owner", owner_short)
            .set("escalate_to", dispatch::escalate_to(&tree, &owner))
            .set("instruction", req.instruction.clone())
            .set("complexity", complexity.as_str())
            .set("model", session.model.clone())
            .set("effort", session.effort.clone())
            .set("budget", session.budget_usd.to_string())
            .set("procedure", rt.procedure.clone());
        let started = fire(
            rt,
            state,
            backend,
            &key,
            &req.errand,
            &format!("{} {} → {}", req.errand, req.plan, owner),
            &owner,
            prompt,
            &rt.cfg.harness.act_cmd,
            &rt.cfg.harness.herdr.act_args,
            vars,
            today,
        );
        outcome.record(started);
    }

    for item in &report.dispatch {
        let key = state::key_plan(&item.plan);
        // The cooldown is the tight loop's backstop: a session that ended
        // without claiming leaves its plan `ready`, and without this it
        // would respawn every tick.
        if !backend.is_busy(&key) {
            if let Some(at) = state.last_attempt_secs(&key) {
                let elapsed = state::now_secs().saturating_sub(at);
                if elapsed < rt.cfg.scheduler.retry_cooldown_secs {
                    if reported.cooling.insert(key.clone()) {
                        note(&format!(
                            "{}: last session ended without claiming — cooling down {}s before retrying",
                            item.plan,
                            rt.cfg.scheduler.retry_cooldown_secs - elapsed
                        ));
                    }
                    continue;
                }
            }
        }
        reported.cooling.remove(&key);
        let vars = tmpl::Vars::new()
            .set("plan", item.plan.clone())
            .set("owner", item.owner_short.clone())
            .set("escalate_to", item.escalate_to.clone())
            .set("automation", item.automation.clone())
            .set("complexity", item.complexity.clone())
            .set("model", item.model.clone())
            .set("effort", item.effort.clone())
            .set("budget", item.budget_usd.to_string())
            .set("procedure", rt.procedure.clone());
        let started = fire(
            rt,
            state,
            backend,
            &key,
            "act",
            &format!("act {} → {}", item.plan, item.owner),
            &item.owner,
            &rt.cfg.prompts["act"],
            &rt.cfg.harness.act_cmd,
            &rt.cfg.harness.herdr.act_args,
            vars,
            today,
        );
        outcome.record(started);
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

        // Parked questions ride the same transports, one tick late at worst.
        // The inbox stays the answering contract; this only shortens the
        // time until somebody looks at it.
        if !rt.channels.is_empty() {
            let fresh: Vec<mcp::Pending> = {
                let mut seen = rt.announced_tickets.lock().unwrap();
                rt.shared
                    .inbox
                    .pending()
                    .into_iter()
                    .filter(|p| seen.insert(p.ticket.clone()))
                    .collect()
            };
            for failure in rt.channels.announce_questions(&fresh) {
                note(&failure);
            }
        }
    }

    {
        let mut status = rt.shared.status.lock().unwrap();
        status.last_pass = Some(today.to_string());
        status.in_flight = backend.in_flight();
        status.runs = state.runs.clone();
        status.warnings = report.warnings.clone();
        status.held = report.held.iter().map(|h| h.plan.clone()).collect();
        status.handoffs = report.handoffs.iter().map(|h| h.plan.clone()).collect();
        if !rt.dry_run {
            if let Err(e) = write_status(&rt.root, &status) {
                note(&format!("status snapshot failed: {e:#}"));
            }
        }
    }

    if !rt.dry_run {
        state.save(&rt.root)?;
    }
    Ok(outcome)
}

/// One rituals pass: everything the cadences owe today. `first` gates the
/// facts worth saying once, since the drain loop may pass again.
fn rituals_pass(
    rt: &Runtime,
    state: &mut State,
    backend: &mut Backend,
    first: bool,
) -> anyhow::Result<Pass> {
    for done in backend.reap() {
        report_exit(&done);
        state.finished(&done.key, done.exit);
        retire(rt, state, &done);
    }

    let tree = Tree::load(Root {
        path: rt.root.clone(),
    })?;
    let reg = registries::load(&tree);
    let today = dates::today();
    let pass = sched::due(&reg, &rt.cfg, state, today);

    if first {
        for row in &pass.unscheduled {
            note(&format!(
                "{row}: cadence has no day count — never fired automatically"
            ));
        }
    }

    let mut outcome = Pass::default();
    for due in pass.due {
        if due.skipped {
            note(&format!(
                "{}: window missed while nothing was running — recorded, not run (catchup = skip)",
                due.task.label()
            ));
            if !rt.dry_run {
                state.fired(&due.task.key(), "ritual", today, None);
            }
            continue;
        }
        let vars = tmpl::Vars::new()
            .set("ritual", due.task.name.clone())
            .set(
                "escalate_to",
                crate::dispatch::escalate_to(&tree, &due.task.executor),
            )
            .set("automation", "")
            .set("executor", due.task.executor.clone())
            .set("procedure", rt.procedure.clone());
        let started = fire(
            rt,
            state,
            backend,
            &due.task.key(),
            "ritual",
            &format!("ritual {} ({})", due.task.name, due.task.executor),
            &due.task.executor,
            &rt.cfg.prompts["ritual"],
            &rt.cfg.harness.ritual_cmd,
            &rt.cfg.harness.herdr.ritual_args,
            vars,
            today,
        );
        outcome.record(started);
    }

    if !rt.dry_run {
        state.save(&rt.root)?;
    }
    Ok(outcome)
}

/// The dispatcher's snapshot, atomically: the read-only server serves this
/// file instead of sharing the process.
fn write_status(root: &Path, status: &Status) -> anyhow::Result<()> {
    let dir = RuntimeConfig::runtime_dir(root);
    std::fs::create_dir_all(&dir)?;
    let tmp = tempfile::NamedTempFile::new_in(&dir)?;
    std::fs::write(tmp.path(), serde_json::to_string_pretty(status)?)?;
    tmp.persist(dir.join(STATUS_FILE))
        .map_err(|e| anyhow::anyhow!("atomic rename failed: {e}"))?;
    Ok(())
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
    backend: &mut Backend,
    key: &str,
    errand: &str,
    label: &str,
    role: &str,
    prompt_template: &str,
    argv_template: &[String],
    herdr_args_template: &[String],
    vars: tmpl::Vars,
    today: Date,
) -> Outcome {
    if backend.is_busy(key) {
        note(&format!(
            "{label}: already in flight — not starting a second"
        ));
        return Outcome::Deferred;
    }
    if backend.already_ran(key) {
        return Outcome::AlreadyRan;
    }
    if backend.capacity() == 0 {
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
            vars = vars.set("mcp", mcp::config_json("127.0.0.1", port, &handle));
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

    // Both backends run from the same rendered prompt; they differ in who
    // carries it. The process backend substitutes it into the argv, so it is
    // one argument no matter what it contains; the herdr backend keeps it
    // aside and submits it to the running agent.
    let launch = match render(
        backend.is_herdr(),
        prompt_template,
        argv_template,
        herdr_args_template,
        &vars,
    ) {
        Ok(launch) => launch,
        Err(e) => {
            note(&format!("{label}: {e:#}"));
            close();
            return Outcome::Failed;
        }
    };

    if rt.dry_run {
        if backend.is_herdr() {
            note(&format!(
                "would run {label} in herdr: {} {} ⇐ {}",
                rt.cfg.harness.herdr.kind,
                spawn::shell_quote(&launch.argv),
                launch.prompt
            ));
        } else {
            note(&format!(
                "would run {label}: {}",
                spawn::shell_quote(&launch.argv)
            ));
        }
        close();
        return Outcome::Started;
    }
    match backend.spawn(key, label, &launch, token.clone()) {
        Ok(log) => {
            note(&format!("started {label} → logs/{log}"));
            // The runtime stamps the acting-role marker (decision 0045): the
            // session finds attribution in place instead of spending a turn
            // creating it. Attribution, not a lock — a failure is said, not
            // fatal.
            if let Err(e) = marker::add(&rt.root, role, &today.to_string(), key) {
                note(&format!("{label}: acting-role marker write failed: {e}"));
            }
            state.fired(key, errand, today, Some(log));
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
/// prompt is one argument no matter what it contains. For herdr the argv is
/// the agent's flags and the prompt rides alongside, unsubstituted into
/// anything.
fn render(
    for_herdr: bool,
    prompt_template: &str,
    argv_template: &[String],
    herdr_args_template: &[String],
    vars: &tmpl::Vars,
) -> anyhow::Result<Launch> {
    let prompt = tmpl::substitute("prompt", &[prompt_template.to_string()], vars)?
        .into_iter()
        .next()
        .expect("one element in, one out");
    if for_herdr {
        return Ok(Launch {
            argv: tmpl::substitute("herdr args", herdr_args_template, vars)?,
            prompt,
        });
    }
    Ok(Launch {
        argv: tmpl::substitute(
            "command",
            argv_template,
            &vars.clone().set("prompt", prompt),
        )?,
        prompt: String::new(),
    })
}

/// Close a finished session's back-channel. A question outstanding when its
/// session exits goes with it: there is nobody left to receive the answer, and
/// leaving it listed would invite an operator to answer into a void.
fn retire(rt: &Runtime, state: &State, done: &spawn::Finished) {
    if let Some(token) = &done.token {
        rt.shared.inbox.retire(token);
    }
    if let Err(e) = marker::remove(&rt.root, &done.key) {
        note(&format!(
            "{}: acting-role marker removal failed: {e}",
            done.label
        ));
    }
    relinquish(rt, state, done);
}

/// Hand an abandoned plan back to the queue. A session sent to advance a plan
/// claims it `ready → active` and owes a verdict — retired, blocked, or a
/// declared handoff. Ending with none of them leaves `active` with nobody
/// holding it: no scan reads that, no tick retries it, and the plan strands
/// until a human notices (observed live, 2026-08-14). `ready` is what the
/// state actually is, so the runtime says so and the next pass picks it up.
///
/// Only `act` sessions: the errand was "advance this plan". A refine session
/// never claimed anything, and demoting a plan a human is driving would be
/// the runtime overruling its operator.
fn relinquish(rt: &Runtime, state: &State, done: &spawn::Finished) {
    if rt.dry_run {
        return;
    }
    let Some(rel) = done.key.strip_prefix("plan:") else {
        return;
    };
    if state.last_errand(&done.key) != Some("act") {
        return;
    }
    let path = rt.root.join(rel);
    if !path.exists() {
        // Retired into the terminal tier, or renamed: either way the session
        // left a verdict and the plan is no longer at this address.
        return;
    }
    match crate::plan_ops::relinquish(&path) {
        Ok(crate::plan_ops::Relinquished::Released) => note(&format!(
            "{rel}: its session ended with the plan still active and nobody holding it — \
             returned to ready; the next pass re-dispatches it"
        )),
        Ok(crate::plan_ops::Relinquished::Parked(handoff)) => note(&format!(
            "{rel}: active, parked on {handoff} — left for its owner, not re-dispatched"
        )),
        Ok(crate::plan_ops::Relinquished::Untouched(_)) => {}
        Err(e) => note(&format!("{rel}: could not read the plan back ({e:#})")),
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
/// something unavailable is refused at startup rather than at the first
/// spawn. Only the active backend's templates count: a `{plugin_dir}` in
/// argv the herdr backend will never render should not refuse its startup.
fn names(cfg: &RuntimeConfig, placeholder: &str) -> bool {
    let uses = |t: &[String]| t.iter().any(|e| e.contains(placeholder));
    let harness = match cfg.harness.backend {
        config::BackendKind::Process => uses(&cfg.harness.act_cmd) || uses(&cfg.harness.ritual_cmd),
        config::BackendKind::Herdr => {
            uses(&cfg.harness.herdr.act_args) || uses(&cfg.harness.herdr.ritual_args)
        }
    };
    harness
        || cfg.prompts.values().any(|p| p.contains(placeholder))
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
    fn claim(root: &Path, file: &str, what: &str) -> anyhow::Result<Pidfile> {
        let dir = RuntimeConfig::runtime_dir(root);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(file);
        if let Ok(text) = std::fs::read_to_string(&path) {
            let pid = text.trim().to_string();
            if alive(&pid) {
                anyhow::bail!(
                    "another {what} is already running on this root (pid {pid}); \
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

/// Where each process is listening, for the commands that talk to them.
/// `trellis inbox` tries the dispatcher first — the sessions live there.
pub const SERVE_ADDRFILE: &str = "serve.addr";
pub const DISPATCH_ADDRFILE: &str = "dispatch.addr";
pub const RITUALS_ADDRFILE: &str = "rituals.addr";

struct Addrfile(PathBuf);

impl Addrfile {
    fn write(root: &Path, file: &str, bind: &str, port: u16) -> anyhow::Result<Addrfile> {
        let dir = RuntimeConfig::runtime_dir(root);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(file);
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
