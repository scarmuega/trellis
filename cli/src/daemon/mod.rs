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
pub mod errands;
pub mod marker;
#[cfg(unix)]
pub mod herdr;
pub mod mcp;
pub mod procedure;
pub mod sched;
pub mod server;
pub mod session;
pub mod state;
pub mod tmpl;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};

use serde::Serialize;

use self::config::RuntimeConfig;
#[cfg(unix)]
use self::herdr::pool::{Disposition, HerdrPool};
use self::session::{Finished, InFlightView, Reason};
use self::state::State;
use crate::dates::{self, Date};
use crate::dispatch::{self, SessionMap};
use crate::gitio::Git;
use crate::graph;
use crate::model::PlanStatus;
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
    /// Errand sessions an operator asked for. Durable (decision 0065):
    /// `.trellis/runtime/errands.json`, saved on every mutation, because the
    /// ask is typed by a human and losing it sends them back to the keyboard.
    /// The loop stays the only spawner; this is what it spawns from.
    pub errands: Mutex<errands::Queue>,
    /// Whether a session's budget cap is actually enforced. Always false
    /// since sessions became interactive panes (`--max-budget-usd` only
    /// works with `--print`), and reported all the same: a number the
    /// operator reads as a ceiling, with nothing holding it, is worse than
    /// saying so. The field stays because the honest answer is worth a wire
    /// field, and because a budget expression that works outside print mode
    /// is one of 0043's named pull-triggers.
    pub budget_enforced: bool,
    /// Ask the loop to pass now. Set when an errand is queued for a plan that
    /// already has a session: `status.in_flight` is only as fresh as the last
    /// pass, so if that session actually ended, the nudged pass reaps it and
    /// the ask starts within a second instead of a tick.
    pub nudge: std::sync::atomic::AtomicBool,
}

/// The name the errand runs under wherever a session is labelled or
/// recorded. Not a `[prompts]` key and not a type an operator picks
/// (decision 0060) — there is one errand, and this is what it is called.
pub const ERRAND: &str = "errand";

/// One operator ask: run this instruction over this plan, at this size.
#[derive(Debug, Clone)]
pub struct ErrandRequest {
    /// The plan rel ("plans/x.md").
    pub plan: String,
    /// The whole of the ask, written at the moment it was wanted.
    pub instruction: String,
    /// Sizing chosen at the call. `None` falls back to the plan's
    /// `complexity:` tier, which is what dispatch would have used.
    pub model: Option<String>,
    pub effort: Option<String>,
}

impl Shared {
    /// Queue an errand and return the entry it became.
    ///
    /// Nothing is displaced (decision 0065): a second ask on the same plan
    /// joins the first and they fire in order. The old behaviour replaced it
    /// and reported the replacement, on the reading that an operator who asks
    /// twice changed their mind — which cost real instructions, because the
    /// commonest reason to ask twice is not knowing whether the first ask
    /// survived.
    pub fn request_errand(&self, request: ErrandRequest, fingerprint: u64) -> errands::Entry {
        let mut queue = self.errands.lock().unwrap();
        let entry = queue.push(
            &request.plan,
            &request.instruction,
            request.model,
            request.effort,
            crate::waits::now_secs(),
            fingerprint,
        );
        self.save_errands(&queue);
        entry
    }

    /// Resolve one ask an operator has looked at: `confirm` re-pins a stale
    /// entry to what the plan says now, `discard` drops it.
    pub fn resolve_errand(&self, id: u64, confirm: Option<u64>) -> Option<errands::Entry> {
        let mut queue = self.errands.lock().unwrap();
        let resolved = match confirm {
            Some(fingerprint) => {
                queue.confirm(id, fingerprint);
                queue.get(id).cloned()
            }
            None => queue.remove(id),
        }?;
        self.save_errands(&queue);
        Some(resolved)
    }

    pub fn errand_view(&self) -> Vec<errands::Entry> {
        self.errands.lock().unwrap().entries.clone()
    }

    pub fn has_errands(&self) -> bool {
        self.errands.lock().unwrap().has_pending()
    }

    /// Persist the queue. Said, never fatal — a queue the daemon can still
    /// serve from memory is better than a pass that refuses to run — but said
    /// loudly, because unlike the state files this one cannot be reconstructed
    /// from the tree.
    pub fn save_errands(&self, queue: &errands::Queue) {
        if let Err(e) = queue.save(&self.root) {
            note(&format!(
                "the errand queue could not be written ({e:#}) — asks are held in memory only \
                 until this clears, and would not survive a restart"
            ));
        }
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
        }
    }
}

/// Whether one plan may carry an errand session right now; `Ok` carries its
/// `owner:`. A human-held owner is no refusal (decision 0057): the request
/// is the holder's own directive, so the session runs as delegated
/// execution — the never-impersonate rule binds the runtime's triggers (the
/// scan, the cadence), not the holder's asks. The drain pass renders the
/// disposition into the prompt as `{delegation}`.
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
    Ok(owner)
}

/// The `{delegation}` value for one errand fire (decision 0057): empty for
/// an agent-held or undeclared holder — the pre-0057 prompt, byte for byte —
/// and the disposition sentence when the owner's holder declares
/// `kind: human`, so the session proceeds under act's delegated-execution
/// branch instead of stopping at never-impersonate.
pub fn delegation_note(tree: &Tree, owner: &str) -> String {
    let owner_short = owner.strip_prefix("org/").unwrap_or(owner);
    if crate::org::holder(tree, owner_short).is_declared_human() {
        "This role's holder is a declared human, and this errand is their own \
         request — delegated execution (act's holder branch, decision 0057): \
         proceed under the mandate at the holder's direction rather than \
         handing off. Their `authority: approve` stays theirs; anything \
         approval-shaped lands in your report as a proposal for them to rule on."
            .to_string()
    } else {
        String::new()
    }
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
        errands: Mutex::new(errands::Queue::default()),
        budget_enforced: false,
        nudge: std::sync::atomic::AtomicBool::new(false),
    });
    let _pidfile = Pidfile::claim(&root, SERVE_PIDFILE, "trellis serve")?;
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
/// Idempotency is the claim the runtime takes for each session as it spawns
/// (decision 0053); the only throttle a tight loop needs on top is the retry
/// cooldown, or a plan whose session dies without a verdict — relinquished
/// back to `ready` — respawns per tick.
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
        // The dispatch process is the sole owner of this file (serve relays
        // to it), so it is the only one that reads it back.
        errands: Mutex::new(errands::Queue::load(&root)),
        budget_enforced: false,
        nudge: std::sync::atomic::AtomicBool::new(false),
    });
    // `--once` takes the same lock as the loop (decision 0053). It skipped
    // it on the reading that a single pass is harmless, but a pass is a pass:
    // it scans the same plans, spawns into the same slots, and writes the
    // same state file. One-shot beside a daemon is two dispatchers on one
    // root, which is the thing the lock exists to refuse.
    let _pidfile = Pidfile::claim(&root, DISPATCH_PIDFILE, "trellis dispatch")?;
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
    let mut backend = connect_pool(&root, &rt.cfg, opts.once, "plan:")?;
    let judge = |key: &str| disposition(&rt, key);

    // A crash removed no marker lines and adopted sessions should keep
    // theirs: reconcile `.trellis/acting-role` against what is actually in
    // flight before the first pass can spawn anything. Every line dropped is
    // a session this runtime stamped and cannot account for, and since the
    // claim precedes the spawn (decision 0053) its plan is sitting `active`
    // with nobody on it — the verdict `relinquish` owes at retire, owed here
    // because the retire never came.
    if !rt.dry_run {
        let live: Vec<String> = backend.in_flight().iter().map(|s| s.key.clone()).collect();
        // The errand half of the same repair: an ask marked dispatched whose
    // session is not in flight belongs back in the queue (decision 0065). The
    // daemon died between the spawn and the conclusion, and the operator is
    // owed the ask rather than an explanation.
    {
        let mut queue = rt.shared.errands.lock().unwrap();
        let requeued = queue.reconcile(&live);
        if !requeued.is_empty() {
            note(&format!(
                "{} queued errand(s) outlived the session started for them — back in the queue",
                requeued.len()
            ));
            rt.shared.save_errands(&queue);
        }
    }
    match marker::reconcile(&root, "plan:", &live) {
            Ok(dropped) => {
                for key in &dropped {
                    relinquish_key(&rt, &state, key);
                }
            }
            Err(e) => note(&format!("acting-role marker reconcile failed: {e}")),
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
            for done in backend.drain(&judge) {
                conclude(&rt, &mut state, &done);
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
        errands: Mutex::new(errands::Queue::default()),
        budget_enforced: false,
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
    let mut backend = connect_pool(&root, &rt.cfg, true, "ritual:")?;
    let judge = |key: &str| disposition(&rt, key);
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
        for done in backend.drain(&judge) {
            conclude(&rt, &mut state, &done);
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
fn shutdown(rt: &Runtime, state: &mut State, backend: &mut HerdrPool) -> anyhow::Result<ExitCode> {
    let (signalled, done) = backend.stop_all();
    if signalled > 0 {
        note(&format!(
            "stopping — asked {signalled} session(s) to stop, waiting for them"
        ));
    }
    for done in done {
        conclude(rt, state, &done);
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
#[derive(PartialEq, Eq, Clone, Copy)]
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
    backend: &mut HerdrPool,
    reported: &mut Reported,
) -> anyhow::Result<Pass> {
    for done in backend.reap(&|key: &str| disposition(rt, key)) {
        conclude(rt, state, &done);
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

    // Operator-requested errand sessions drain first (decisions 0048, 0051,
    // 0060): an explicit ask outranks the scheduled scan for the same slots.
    // Each is one-shot — re-validated against this pass's tree, then fired or
    // dropped with its reason, never carried to a later pass. No cooldown:
    // the cooldown throttles unattended respawns, and this is the attended
    // case.
    // The queue is drained by *reading* it, never by taking it (decision
    // 0065). An ask leaves only three ways: it starts a session, its plan
    // refuses it outright, or its author discards it. A full fleet, a busy
    // plan, and a dispatcher restart all leave it exactly where it is.
    let errand_prompt = config::errand_prompt();
    let queued: Vec<errands::Entry> = rt
        .shared
        .errands
        .lock()
        .unwrap()
        .pending()
        .into_iter()
        .cloned()
        .collect();
    for req in queued {
        let owner = match validate_errand(&tree, &req.plan) {
            Ok(owner) => owner,
            Err(refusal) => {
                // Structural refusals are terminal — the plan is gone,
                // archived, or unowned, and no later pass changes that.
                note(&format!("{ERRAND} dropped — {}", refusal.describe(&req.plan)));
                drop_errand(rt, req.id);
                continue;
            }
        };
        // The ask was written about a plan. If that plan no longer says what
        // it said, the ask is held rather than fired or dropped: only its
        // author can say whether it survived the rewrite.
        match crate::plan_ops::fingerprint(&rt.root.join(&req.plan)) {
            Ok(current) if current != req.fingerprint => {
                note(&format!(
                    "{ERRAND} {} — the plan changed since this was asked; held for review",
                    req.plan
                ));
                let mut queue = rt.shared.errands.lock().unwrap();
                queue.mark_stale(req.id);
                rt.shared.save_errands(&queue);
                continue;
            }
            // Unreadable reads as unchanged: a plan caught mid-write must not
            // cost the operator their ask.
            Ok(_) | Err(_) => {}
        }
        let owner_short = owner.strip_prefix("org/").unwrap_or(&owner).to_string();
        // The tier the plan's `complexity:` resolves to is the default; what
        // the operator chose at the call wins over it (decision 0060), which
        // is the one thing that genuinely varies per ask.
        let (complexity, session) =
            dispatch::plan_session(&tree, &rt.shared.sessions, &req.plan);
        let model = req.model.clone().unwrap_or_else(|| session.model.clone());
        let effort = req.effort.clone().unwrap_or_else(|| session.effort.clone());
        let key = state::key_plan(&req.plan);
        let vars = tmpl::Vars::new()
            .set("plan", req.plan.clone())
            .set("owner", owner_short)
            .set("escalate_to", dispatch::escalate_to(&tree, &owner))
            .set("instruction", req.instruction.clone())
            .set("delegation", delegation_note(&tree, &owner))
            .set("complexity", complexity.as_str())
            .set("model", model)
            .set("effort", effort)
            .set("budget", session.budget_usd.to_string())
            .set("skills", crate::skills::for_plan(&tree, &req.plan, &owner))
            .set("mandate", crate::org::mandate_block(&tree, &owner))
            .set("holder", crate::org::holder_block(&tree, &owner))
            .set("procedure", rt.procedure.clone());
        // Marked before the spawn, and saved, for the reason `fire` orders its
        // own writes that way: a crash during `agent.start` must leave a
        // record a later startup can reconcile, not an ask nobody owns.
        {
            let mut queue = rt.shared.errands.lock().unwrap();
            queue.mark_dispatched(req.id, &key);
            rt.shared.save_errands(&queue);
        }
        let started = fire(
            rt,
            state,
            backend,
            &key,
            ERRAND,
            &format!("{ERRAND} {} → {}", req.plan, owner),
            &owner,
            &errand_prompt,
            &rt.cfg.harness.herdr.act_args,
            vars,
            today,
        );
        // Deferred is the case this whole change exists for: no free slot, or
        // a session already on this plan. The ask goes back in the queue and
        // the next pass tries again — where it used to be logged as "still
        // due" and silently thrown away.
        if started != Outcome::Started {
            let mut queue = rt.shared.errands.lock().unwrap();
            if started == Outcome::Failed {
                queue.remove(req.id);
            } else {
                queue.confirm(req.id, req.fingerprint);
            }
            rt.shared.save_errands(&queue);
        }
        outcome.record(started);
    }

    for item in &report.dispatch {
        let key = state::key_plan(&item.plan);
        // The cooldown is the tight loop's backstop: a session that ended
        // without a verdict has its plan relinquished to `ready`, and
        // without this it would respawn every tick.
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
            .set(
                "skills",
                crate::skills::for_plan(&tree, &item.plan, &item.owner),
            )
            .set("mandate", crate::org::mandate_block(&tree, &item.owner))
            .set("holder", crate::org::holder_block(&tree, &item.owner))
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
    backend: &mut HerdrPool,
    first: bool,
) -> anyhow::Result<Pass> {
    for done in backend.reap(&|key: &str| disposition(rt, key)) {
        conclude(rt, state, &done);
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
            .set(
                "skills",
                crate::skills::for_ritual(&tree, &due.task.executor),
            )
            .set(
                "mandate",
                crate::org::mandate_block(&tree, &due.task.executor),
            )
            .set(
                "holder",
                crate::org::holder_block(&tree, &due.task.executor),
            )
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
    backend: &mut HerdrPool,
    key: &str,
    errand: &str,
    label: &str,
    role: &str,
    prompt_template: &str,
    args_template: &[String],
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

    // Herdr owns the executable and takes the prompt through its own
    // channel, so what renders is a flag array and a prompt that is never
    // substituted into anything.
    let (args, prompt) = match render(prompt_template, args_template, &vars) {
        Ok(rendered) => rendered,
        Err(e) => {
            note(&format!("{label}: {e:#}"));
            close();
            return Outcome::Failed;
        }
    };

    if rt.dry_run {
        note(&format!(
            "would run {label} in herdr: {} {} ⇐ {}",
            rt.cfg.harness.herdr.kind,
            session::shell_quote(&args),
            prompt
        ));
        close();
        return Outcome::Started;
    }

    // The order below is the crash story, and it is why these three writes
    // are not in the order they read in.
    //
    // Claimed before the session exists, not on its first turn (decision
    // 0053). Until now the flip was the taker's, which left minutes between
    // spawn and claim in which the plan still read `ready` and any scan —
    // this daemon's next pass, another process's, a `--once` — could dispatch
    // it again. Two dispatchers did exactly that (observed live, 2026-08-14).
    let claimed = claim_to_dispatch(rt, key, errand);

    // Then the marker and the fired stamp, both *before* the spawn, because
    // between them they are what a later startup needs to recognise this
    // claim as abandoned: `marker::reconcile` finds the line, and
    // `relinquish_key` gates on the errand this records. Written after the
    // spawn — as they were — a crash during `agent.start`, which can take a
    // full minute, left an `active` plan that no reconcile would ever see.
    // Now the exposure is the two syscalls between the claim and here.
    if let Err(e) = marker::add(&rt.root, role, &today.to_string(), key) {
        // Attribution, not a lock (decision 0045): the session finds it in
        // place instead of spending a turn creating it. A failure is said,
        // not fatal.
        note(&format!("{label}: acting-role marker write failed: {e}"));
    }
    state.fired(key, errand, today, None);
    if let Err(e) = state.save(&rt.root) {
        note(&format!("{label}: could not record the attempt ({e:#})"));
    }

    match backend.spawn(key, label, &args, &prompt, token.clone()) {
        Ok(log) => {
            note(&format!("started {label} → logs/{log}"));
            state.set_log(key, log);
            Outcome::Started
        }
        Err(e) => {
            note(&format!("{label}: {e:#}"));
            // Nothing started, so nothing holds the plan. A claim left
            // standing here is the strand `relinquish` exists to prevent,
            // and no conclusion will ever come to undo it: a failed spawn
            // never enters the pool. The fired stamp deliberately stays —
            // an attempt was made, and the retry cooldown should pace the
            // next one.
            if let Err(e) = marker::remove(&rt.root, key) {
                note(&format!("{label}: acting-role marker removal failed: {e}"));
            }
            if let Some(path) = &claimed {
                if let Err(e) =
                    crate::plan_ops::flip(path, &[PlanStatus::Active], PlanStatus::Ready)
                {
                    note(&format!("{label}: could not release the claim ({e:#})"));
                }
            }
            close();
            Outcome::Failed
        }
    }
}

/// Take the plan for the session about to run it, returning the path claimed
/// so a failed spawn can put it back.
///
/// This is the dispatch guard, and the plan is what carries it: `ready` is
/// the queue and `active` means somebody is on it (decision 0052), so the
/// flip is the one in-flight fact every reader already consults — the next
/// scan, another process's scan, `--once`, the board. A guard in the
/// daemon's memory would have to be shared with every other daemon to mean
/// anything; this one is on disk because that is where the queue is.
///
/// Only `act`, and only from `ready` — the same pair `relinquish` tests on
/// the other end, because they are the two halves of one rule. An operator
/// errand aimed at a plan in any other state is an attended act and takes it
/// as it finds it; a `refine` never claims at all.
fn claim_to_dispatch(rt: &Runtime, key: &str, errand: &str) -> Option<PathBuf> {
    if rt.dry_run || errand != "act" {
        return None;
    }
    let rel = key.strip_prefix("plan:")?;
    let path = rt.root.join(rel);
    if crate::plan_ops::current_status(&path).ok()? != PlanStatus::Ready {
        return None;
    }
    // A claim is a fresh attempt, so a spent wait is cleared with the spent
    // handoff `flip` drops — same rule, and the same reason: a lease left
    // over from the last taker would buy this session a patience it never
    // asked for.
    clear_wait(rt, key);
    match crate::plan_ops::flip(&path, &[PlanStatus::Ready], PlanStatus::Active) {
        Ok(_) => Some(path),
        // Said, not fatal: the session is still the one that does the work,
        // and refusing to spawn over an unwritable plan would strand it.
        // The cooldown bounds the re-dispatch this leaves possible.
        Err(e) => {
            note(&format!(
                "{rel}: could not claim it for the session ({e:#})"
            ));
            None
        }
    }
}

/// Drop one ask the tree itself has refused — the plan is gone, archived, or
/// unowned, and no later pass changes that.
fn drop_errand(rt: &Runtime, id: u64) {
    let mut queue = rt.shared.errands.lock().unwrap();
    queue.remove(id);
    rt.shared.save_errands(&queue);
}

/// Drop any wait lease behind a session key. Said, never fatal, and a no-op
/// for a ritual: the lease is a liveness claim, and the plan's own status is
/// what the runtime falls back on when there is none.
fn clear_wait(rt: &Runtime, key: &str) {
    let Some(rel) = key.strip_prefix("plan:") else {
        return;
    };
    if let Err(e) = crate::waits::clear(&rt.root, rel) {
        note(&format!("{rel}: could not clear the wait ({e})"));
    }
}

/// The agent's flags and the prompt it will be handed, each substituted on
/// its own. The prompt is never substituted *into* anything: herdr submits
/// it to the running agent, so it is never an argument and never has to
/// survive a quoting layer.
fn render(
    prompt_template: &str,
    args_template: &[String],
    vars: &tmpl::Vars,
) -> anyhow::Result<(Vec<String>, String)> {
    let prompt = tmpl::substitute("prompt", &[prompt_template.to_string()], vars)?
        .into_iter()
        .next()
        .expect("one element in, one out");
    Ok((tmpl::substitute("herdr args", args_template, vars)?, prompt))
}

/// Open the session pool. Herdr carries every session (decision 0061), so a
/// server that is configured-and-dead is a refusal at startup rather than a
/// degraded run — the `{plugin_dir}` posture, applied to the one dependency
/// the runtime now has.
#[cfg(unix)]
fn connect_pool(
    root: &Path,
    cfg: &RuntimeConfig,
    once: bool,
    key_prefix: &'static str,
) -> anyhow::Result<HerdrPool> {
    if cfg.harness.herdr.idle_grace_secs < config::SHORT_GRACE_SECS {
        note(&format!(
            "harness.herdr.idle_grace_secs is {}s — short enough that a slow turn boundary \
             could read as a stall and be recycled",
            cfg.harness.herdr.idle_grace_secs
        ));
    }
    let pool = HerdrPool::connect(
        root,
        &cfg.harness.herdr,
        cfg.scheduler.max_concurrent,
        key_prefix,
    )?;
    Ok(if once { pool.once_per_task() } else { pool })
}

/// What the runtime knows, from disk, about the plan behind a session key —
/// the whole of how a session's end is decided (decision 0061).
///
/// Read straight from the file rather than from the pass's tree snapshot:
/// this is asked of a pane that has *just* settled, and the session's last
/// act is usually the very write being judged. It is one small read per
/// settled pane per tick.
///
/// Two soft edges, both deliberate. Frontmatter that will not parse reads as
/// `Held`, so a session caught mid-write is never retired out from under
/// itself; the grace still bounds it. And a plan that would read `Held` is
/// checked for a declared wait first — the session's own statement that it is
/// minding work outliving its turn — which suspends the recycle for as long
/// as the lease is live, capped by `max_wait_secs` from the moment it was
/// written.
#[cfg(unix)]
fn disposition(rt: &Runtime, key: &str) -> Disposition {
    let Some(rel) = key.strip_prefix("plan:") else {
        return Disposition::NoPlan;
    };
    let root = &rt.root;
    let path = root.join(rel);
    if !path.exists() {
        // Retired into the terminal tier, or renamed: either way the session
        // left a verdict and the plan is no longer at this address.
        return Disposition::Verdict;
    }
    match crate::plan_ops::current_status(&path) {
        Ok(PlanStatus::Active) => match crate::plan_ops::handoff(&path) {
            // Parked on something declared, which is an answer — and one
            // that frees the slot, because a pane waiting on a PR is a pane
            // nobody is typing in.
            Ok(Some(_)) => Disposition::Verdict,
            // No verdict — but a session minding its own build is not a
            // stall, and it is the only one that can say so. A handoff is
            // the wrong word for it: that parks the plan on something
            // *somebody else* moves, and closes the pane the build is
            // running in.
            _ => match held_or_waiting(rt, rel) {
                Some(what) => Disposition::Waiting(what),
                None => Disposition::Held,
            },
        },
        Ok(_) => Disposition::Verdict,
        Err(_) => Disposition::Held,
    }
}

/// The live wait a session declared on this plan, if any. Absent, lapsed, and
/// unreadable are one answer: no wait, and the plan's own status decides —
/// which is never worse than half a lease.
#[cfg(unix)]
fn held_or_waiting(rt: &Runtime, rel: &str) -> Option<String> {
    let cap = rt.cfg.harness.herdr.max_wait_secs;
    let wait = crate::waits::live(&rt.root, rel, crate::waits::now_secs(), cap)?;
    Some(if wait.what.is_empty() {
        "work that outlives the turn".to_string()
    } else {
        wait.what
    })
}

/// Everything a session's end owes the rest of the runtime: say so, record
/// it, close the back-channel, drop the marker line, and hand back a plan
/// nobody accounted for.
///
/// A `Detached` session is the exception at every step that touches the
/// tree: it is still running, so its marker line stays and its plan is left
/// alone — the next daemon adopts the pane and the line together.
fn conclude(rt: &Runtime, state: &mut State, done: &Finished) {
    match done.reason {
        Reason::Verdict => note(&format!("finished {}", done.label)),
        Reason::Settled => note(&format!("finished {}", done.label)),
        Reason::Recycled => note(&format!(
            "{} left its plan unaccounted for — handing it back",
            done.label
        )),
        Reason::Gone => note(&format!("{} — its pane is gone", done.label)),
        Reason::ServerLost => note(&format!("{} — herdr went away under it", done.label)),
        Reason::Detached => note(&format!("{} — left running", done.label)),
        Reason::Stopped => note(&format!("{} — stopped", done.label)),
    }
    state.concluded(&done.key, done.reason);
    if let Some(token) = &done.token {
        rt.shared.inbox.retire(token);
    }
    if done.reason == Reason::Detached {
        return;
    }
    // A wait outlives its turn, never its session: whatever the lease still
    // said, the session that took it is over. Expiry would get here on its
    // own; this is so the next taker never inherits one.
    clear_wait(rt, &done.key);
    // The ask this session was started for is answered by the session having
    // happened, whatever it concluded — an errand is one ask, not a standing
    // order, and a plan handed back is re-scanned as an `act` in any case.
    {
        let mut queue = rt.shared.errands.lock().unwrap();
        if queue.remove_dispatched(&done.key).is_some() {
            rt.shared.save_errands(&queue);
        }
    }
    if let Err(e) = marker::remove(&rt.root, &done.key) {
        note(&format!(
            "{}: acting-role marker removal failed: {e}",
            done.label
        ));
    }
    relinquish_key(rt, state, &done.key);
}

/// Hand an abandoned plan back to the queue. A session sent to advance a plan
/// claims it `ready → active` and owes a verdict — retired, blocked, passed
/// on, or a declared handoff. Ending with none of them leaves `active` with
/// nobody holding it: no scan reads that, no tick retries it, and the plan
/// strands until a human notices (observed live, 2026-08-14). `ready` is what
/// the state actually is, so the runtime says so and the next pass picks it
/// up.
///
/// By key, so startup can hand back the plans of sessions this runtime
/// stamped and can no longer account for (decision 0053). The key having
/// carried a daemon marker line is what makes it ours to judge: an
/// interactive claim writes the keyless two-line form and is never in this
/// set.
///
/// Only `act` sessions: the errand was "advance this plan". An errand
/// session never claimed anything, and demoting a plan a human is driving
/// would be the runtime overruling its operator.
fn relinquish_key(rt: &Runtime, state: &State, key: &str) {
    if rt.dry_run {
        return;
    }
    let Some(rel) = key.strip_prefix("plan:") else {
        return;
    };
    if state.last_errand(key) != Some("act") {
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

/// The daemon's own log line. Stdout is the supervisor's to capture; session
/// output goes to its own file.
pub fn note(message: &str) {
    println!("[trellis] {message}");
}

/// Whether any template names a placeholder — asked of the ones the daemon
/// must be able to *supply* before the first tick, so a template that names
/// something unavailable is refused at startup rather than at the first
/// spawn.
fn names(cfg: &RuntimeConfig, placeholder: &str) -> bool {
    let uses = |t: &[String]| t.iter().any(|e| e.contains(placeholder));
    uses(&cfg.harness.herdr.act_args)
        || uses(&cfg.harness.herdr.ritual_args)
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
///
/// The claim is a `flock` held for the process's lifetime, not a
/// read-the-pid-then-write-it (decision 0053). Two dispatchers ran on one
/// root for seven hours (observed live, 2026-08-14) and the check-then-write
/// form is why: it is not atomic, so two daemons starting together both pass
/// it, and asking `kill -0` whether a *recorded* pid is alive says nothing
/// about a daemon that never recorded one. The kernel releases a `flock` when
/// its holder dies, however it dies, so there is no staleness to adjudicate
/// and no file for an operator to remove by hand. The pid still goes in the
/// file — the board's liveness overlay and this error message read it — but
/// it is what the lock says about itself, never the lock.
#[derive(Debug)]
struct Pidfile {
    path: PathBuf,
    /// Dropped last, releasing the lock. Held open for exactly that.
    #[cfg(unix)]
    _handle: std::fs::File,
}

impl Pidfile {
    #[cfg(unix)]
    fn claim(root: &Path, file: &str, what: &str) -> anyhow::Result<Pidfile> {
        use std::io::{Seek, Write};
        use std::os::unix::fs::MetadataExt;
        use std::os::unix::io::AsRawFd;

        let dir = RuntimeConfig::runtime_dir(root);
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(file);

        // Twice, because the holder unlinks on the way out: winning the lock
        // on an inode that is no longer the one at `path` means we raced a
        // `Drop`, and the file worth locking is the one that replaced it.
        // Without the recheck that race hands the lock to two processes.
        for _ in 0..2 {
            let handle = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                // Never truncate on open: a contender arriving mid-claim
                // should read the incumbent's pid, not an empty file.
                .truncate(false)
                .open(&path)?;
            // SAFETY: `flock` on a descriptor this scope owns. `LOCK_NB`
            // makes it a test, never a wait — a daemon that cannot have the
            // root says so and exits rather than queueing behind one.
            let taken = unsafe { libc::flock(handle.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if taken != 0 {
                let pid = std::fs::read_to_string(&path)
                    .map(|t| t.trim().to_string())
                    .ok()
                    .filter(|p| !p.is_empty())
                    .unwrap_or_else(|| "unknown".into());
                anyhow::bail!(
                    "another {what} is already running on this root (pid {pid}); stop it first"
                );
            }
            let ours = match (handle.metadata(), std::fs::metadata(&path)) {
                (Ok(held), Ok(named)) => held.dev() == named.dev() && held.ino() == named.ino(),
                // The path is gone: the holder we raced took it with them.
                _ => false,
            };
            if !ours {
                continue;
            }
            let mut handle = handle;
            handle.set_len(0)?;
            handle.rewind()?;
            handle.write_all(format!("{}\n", std::process::id()).as_bytes())?;
            handle.flush()?;
            return Ok(Pidfile {
                path,
                _handle: handle,
            });
        }
        anyhow::bail!(
            "could not claim {} for {what} — another one is starting or stopping on this root",
            path.display()
        )
    }

    /// Without `flock`, the advisory form is all there is: read the recorded
    /// pid, believe it if the OS says it is alive. The ownership check in
    /// `Drop` still applies, which is the half of the fix that does not need
    /// a kernel lock.
    #[cfg(not(unix))]
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
        Ok(Pidfile { path })
    }

    /// Whether the file at the path still records this process. Under
    /// `flock` it cannot say otherwise while the lock is held; the check
    /// earns its keep on the platform without one, and costs a read here.
    fn is_ours(&self) -> bool {
        std::fs::read_to_string(&self.path)
            .map(|t| t.trim() == std::process::id().to_string())
            .unwrap_or(false)
    }
}

impl Drop for Pidfile {
    fn drop(&mut self) {
        // Only ever our own file. This removed whatever was at the path
        // unconditionally, which made any exiting dispatcher the one that
        // deleted a live daemon's lock (decision 0053).
        if self.is_ours() {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Which process holds which root. Written under the runtime directory
/// beside the addrfiles.
pub const SERVE_PIDFILE: &str = "serve.pid";
pub const DISPATCH_PIDFILE: &str = "dispatch.pid";

/// Whether a dispatcher is live on this root, for the read-only server's
/// overlay (decision 0046). Existence is not liveness: a clean exit removes
/// the file but a crash leaves it, and what a dispatcher actually holds is
/// the `flock`, not the path — so the recorded pid has to be asked.
pub fn dispatch_running(root: &Path) -> bool {
    std::fs::read_to_string(RuntimeConfig::runtime_dir(root).join(DISPATCH_PIDFILE))
        .map(|t| alive(t.trim()))
        .unwrap_or(false)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn pidfile_path(root: &Path) -> PathBuf {
        RuntimeConfig::runtime_dir(root).join(DISPATCH_PIDFILE)
    }

    fn shared_for_queue(root: &Path) -> Shared {
        Shared {
            status: Mutex::new(Status::default()),
            root: root.to_path_buf(),
            sessions: SessionMap::default(),
            inbox: mcp::Inbox::default(),
            overlay_status: false,
            dispatches: true,
            errands: Mutex::new(errands::Queue::default()),
            budget_enforced: true,
            nudge: std::sync::atomic::AtomicBool::new(false),
        }
    }

    fn ask(plan: &str, instruction: &str) -> ErrandRequest {
        ErrandRequest {
            plan: plan.to_string(),
            instruction: instruction.to_string(),
            model: None,
            effort: None,
        }
    }

    /// Nothing an operator types is displaced (decision 0065). A second ask on
    /// one plan joins the first and they fire in order — the old rule replaced
    /// it and reported the replacement, which cost real instructions, because
    /// the commonest reason to ask twice is not knowing whether the first ask
    /// survived.
    #[test]
    fn a_second_ask_for_one_plan_joins_the_first_rather_than_replacing_it() {
        let dir = tempfile::TempDir::new().unwrap();
        let shared = shared_for_queue(dir.path());

        let first = shared.request_errand(ask("plans/a.md", "split the scope"), 1);
        shared.request_errand(ask("plans/b.md", "another plan, another ask"), 1);
        let second = shared.request_errand(ask("plans/a.md", "actually, tighten it"), 1);
        assert_ne!(first.id, second.id);

        let queued = shared.errand_view();
        assert_eq!(queued.len(), 3, "three asks were made, so three are owed");
        let on_a: Vec<_> = queued.iter().filter(|e| e.plan == "plans/a.md").collect();
        assert_eq!(
            on_a.iter().map(|e| e.instruction.as_str()).collect::<Vec<_>>(),
            vec!["split the scope", "actually, tighten it"],
            "and they run in the order they were asked"
        );

        // And it is on disk the moment it is made, so a restart owes nothing.
        let reloaded = errands::Queue::load(dir.path());
        assert_eq!(reloaded.pending().len(), 3);
    }

    /// The only ways an ask leaves the queue unfired: its author discards it,
    /// or the tree itself refuses it. `confirm` is what re-pins a held one.
    #[test]
    fn an_ask_leaves_the_queue_only_by_running_or_by_being_discarded() {
        let dir = tempfile::TempDir::new().unwrap();
        let shared = shared_for_queue(dir.path());
        let entry = shared.request_errand(ask("plans/a.md", "split the scope"), 7);

        assert!(shared.resolve_errand(entry.id, Some(9)).is_some());
        assert_eq!(shared.errand_view()[0].fingerprint, 9, "confirm re-pins it");

        assert!(shared.resolve_errand(entry.id, None).is_some());
        assert!(shared.errand_view().is_empty());
        assert!(
            shared.resolve_errand(entry.id, None).is_none(),
            "and an ask that is already gone is a 404, not a second discard"
        );
    }

    /// The claim two dispatchers shared a root through (decision 0053). The
    /// second must be refused whatever the first recorded, and — since
    /// `flock` binds to the open file description, not the process — one
    /// process asking twice is the same question the kernel answers.
    #[test]
    #[cfg(unix)]
    fn a_second_claim_on_one_root_is_refused() {
        let dir = tempfile::TempDir::new().unwrap();
        let held = Pidfile::claim(dir.path(), DISPATCH_PIDFILE, "trellis dispatch").unwrap();
        let refused = Pidfile::claim(dir.path(), DISPATCH_PIDFILE, "trellis dispatch").unwrap_err();
        assert!(refused.to_string().contains("already running"), "{refused}");
        assert!(
            held.is_ours(),
            "the refused claim left the incumbent's file alone"
        );
        assert!(dispatch_running(dir.path()), "and the root reads as held");

        drop(held);
        assert!(!pidfile_path(dir.path()).exists(), "a clean exit releases");
        assert!(!dispatch_running(dir.path()));

        Pidfile::claim(dir.path(), DISPATCH_PIDFILE, "trellis dispatch")
            .expect("the root is free again once the holder is gone");
    }

    /// `Drop` removed the file unconditionally, so any exiting dispatcher
    /// deleted whatever lock was recorded — including a live daemon's. The
    /// pid in the file is the test, and a file recording somebody else
    /// survives.
    #[test]
    fn an_exiting_holder_removes_only_its_own_file() {
        let dir = tempfile::TempDir::new().unwrap();
        let held = Pidfile::claim(dir.path(), DISPATCH_PIDFILE, "trellis dispatch").unwrap();
        std::fs::write(pidfile_path(dir.path()), "999999\n").unwrap();
        assert!(!held.is_ours());

        drop(held);
        assert!(
            pidfile_path(dir.path()).exists(),
            "somebody else's lock is not ours to remove"
        );
        assert_eq!(
            std::fs::read_to_string(pidfile_path(dir.path())).unwrap(),
            "999999\n"
        );
    }

    /// A stale file is not a held root: `serve` reads liveness off the
    /// recorded pid, because a crash leaves the file behind and only a clean
    /// exit removes it.
    #[test]
    fn a_pidfile_left_by_a_dead_process_is_not_a_live_dispatcher() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(RuntimeConfig::runtime_dir(dir.path())).unwrap();
        std::fs::write(pidfile_path(dir.path()), "999999\n").unwrap();
        assert!(!dispatch_running(dir.path()));
        Pidfile::claim(dir.path(), DISPATCH_PIDFILE, "trellis dispatch")
            .expect("and it is no obstacle to claiming the root");
    }

    /// Serve and dispatch hold different files, so one never refuses the
    /// other — the lock is per process kind, not per root.
    #[test]
    fn serve_and_dispatch_do_not_contend() {
        let dir = tempfile::TempDir::new().unwrap();
        let _dispatch = Pidfile::claim(dir.path(), DISPATCH_PIDFILE, "trellis dispatch").unwrap();
        let _serve = Pidfile::claim(dir.path(), SERVE_PIDFILE, "trellis serve").unwrap();
    }
}
