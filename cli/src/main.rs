//! `trellis` — deterministic kernel for Trellis domain roots.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};

use trellis::daemon;
use trellis::dates;
use trellis::dispatch::{self, SessionMap};
use trellis::escalate::{self, NewEscalation};
use trellis::facts;
use trellis::fmedit;
use trellis::gitio::Git;
use trellis::graph;
use trellis::lint;
use trellis::model::PlanStatus;
use trellis::plan_ops;
use trellis::readiness;
use trellis::refs;
use trellis::registries;
use trellis::root::Root;
use trellis::scaffold;
use trellis::tree::{Kind, Tree};
use trellis::views;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Json,
}

#[derive(Parser)]
#[command(
    name = "trellis",
    version,
    about = "Deterministic kernel for Trellis domain roots"
)]
struct Cli {
    /// Operate on this root (or any directory inside it) instead of the cwd
    #[arg(short = 'C', long = "root", global = true, value_name = "DIR")]
    root: Option<PathBuf>,
    /// Output format for read commands
    #[arg(long, global = true, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Read spec/template from a live plugin checkout instead of the
    /// embedded copy (flag > $CLAUDE_PLUGIN_ROOT > embedded)
    #[arg(long, global = true, value_name = "DIR", env = "CLAUDE_PLUGIN_ROOT")]
    plugin_root: Option<PathBuf>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Print the detected Trellis root
    Root {
        /// Exit code only
        #[arg(long)]
        quiet: bool,
    },
    /// Binary and embedded spec version
    Version,
    /// Run the mechanical conventions-lint items; judgment items are
    /// reported as skipped
    Lint {
        /// Promote warnings into the failing set
        #[arg(long)]
        strict: bool,
        /// Run only these items (comma-separated, e.g. 1,4,22)
        #[arg(long, value_delimiter = ',')]
        items: Option<Vec<u8>>,
        /// Restrict findings to these paths
        paths: Vec<String>,
    },
    /// Census of the artifacts discovery took in, and what it left out
    Tree {
        /// Only artifacts of this kind (as `trellis show` spells it)
        #[arg(long, value_name = "KIND")]
        kind: Option<String>,
        /// One root-relative path per line, no drawing — for piping
        #[arg(long)]
        flat: bool,
        /// Restrict to these paths
        paths: Vec<String>,
    },
    /// Mechanical share of the plan-readiness gate
    Readiness { plan: String },
    /// Computed facts about one artifact
    Show { artifact: String },
    /// One computed value (class | band | dwell | held | status)
    Query { what: QueryWhat, artifact: String },
    /// Resolve any ref form against this root
    Resolve { r#ref: String },
    /// Plan lifecycle
    Plan {
        #[command(subcommand)]
        cmd: PlanCmd,
    },
    /// File terminal artifacts into archive/ — the terminal tier
    Archive {
        /// The artifact to file. Omit with --sweep.
        artifact: Option<String>,
        /// Every terminal artifact colder than the declared horizon
        #[arg(long)]
        sweep: bool,
        /// Report what would move; move nothing
        #[arg(long)]
        dry_run: bool,
    },
    /// Escalation records (fenced yaml under ## Escalations)
    Escalate {
        #[command(subcommand)]
        cmd: EscalateCmd,
    },
    /// Plan-dispatch scan (the workflow's deterministic brain)
    Dispatch {
        #[command(subcommand)]
        cmd: DispatchCmd,
    },
    /// Generated views (board | codeowners | tags | orgchart | escalations)
    View {
        name: String,
        /// Write to the canonical path (or --out) instead of stdout
        #[arg(long)]
        write: bool,
        #[arg(long, value_name = "PATH")]
        out: Option<PathBuf>,
        /// Regenerate and diff against the written file; exit 1 on drift
        #[arg(long)]
        check: bool,
    },
    /// Frontmatter plumbing (format-preserving)
    Fm {
        #[command(subcommand)]
        cmd: FmCmd,
    },
    /// Questions the running sessions are waiting on you to answer
    Inbox {
        #[command(subcommand)]
        cmd: Option<InboxCmd>,
        /// Block until a session asks something
        #[arg(long)]
        watch: bool,
        /// Reach a daemon at this host:port instead of the one serving this
        /// root
        #[arg(long, value_name = "ADDR")]
        addr: Option<String>,
    },
    /// PreToolUse hook mode: stdin JSON, always exits 0, fails open
    Gate,
    /// Scaffold a domain instance from the embedded template
    Scaffold {
        dir: PathBuf,
        /// Owner role for <owner> placeholders (founder or org/founder)
        #[arg(long, default_value = "founder")]
        owner: String,
    },
    /// Run the local runtime: rituals on cadence, plan dispatch, and a
    /// read-only board and API over this root
    Serve {
        /// Listen on this port (0 asks the OS; the chosen port is printed)
        #[arg(long)]
        port: Option<u16>,
        /// Listen on this address — anything but loopback exposes an
        /// unauthenticated read surface
        #[arg(long, value_name = "ADDR")]
        bind: Option<String>,
        /// Runtime config (default: runtime.toml at the root)
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        /// One scheduling pass, wait for the sessions it starts, exit
        #[arg(long)]
        once: bool,
        /// Report what would be spawned; spawn nothing, record nothing
        #[arg(long)]
        dry_run: bool,
        /// Scheduler and dispatcher only, no serving surface
        #[arg(long)]
        no_http: bool,
        /// Seconds between scheduling passes
        #[arg(long, value_name = "N")]
        tick_secs: Option<u64>,
        /// Sessions that may run at once
        #[arg(long, value_name = "N")]
        max_concurrent: Option<usize>,
        /// Override a tier's session: tier=model:effort:budget
        #[arg(long, value_name = "SPEC")]
        map: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum QueryWhat {
    Class,
    Band,
    Dwell,
    Held,
    Status,
}

#[derive(Subcommand)]
enum PlanCmd {
    /// Census of plans/ with computed hold state
    List {
        #[arg(long)]
        status: Option<String>,
        /// Only held plans (ready with unsatisfied awaits)
        #[arg(long)]
        held: bool,
        /// Include plans filed in the terminal tier (archive/)
        #[arg(long)]
        archived: bool,
    },
    /// draft → ready, gated on the mechanical readiness pass
    Release {
        plan: String,
        #[arg(long)]
        force: bool,
    },
    /// ready → active (the taker's claim)
    Claim { plan: String },
    /// → blocked, writing the open escalation record the status owes
    Block {
        plan: String,
        #[arg(long)]
        asks: String,
        #[arg(long)]
        attempted: Option<String>,
        #[arg(long)]
        blocked: Option<String>,
        /// Raising role; defaults to the plan's owner
        #[arg(long)]
        by: Option<String>,
    },
    /// blocked → ready (warns if open records remain)
    Unblock { plan: String },
    /// → retired — the owner's verdict; releases awaits: dependents
    Retire { plan: String },
}

#[derive(Subcommand)]
enum EscalateCmd {
    Add {
        artifact: String,
        #[arg(long)]
        by: String,
        /// Defaults to the raiser's escalate-to:, else the artifact's owner
        #[arg(long)]
        to: Option<String>,
        #[arg(long)]
        asks: String,
        #[arg(long)]
        attempted: Option<String>,
        #[arg(long)]
        blocked: Option<String>,
    },
    List {
        /// Include resolved records
        #[arg(long)]
        all: bool,
    },
    Resolve {
        artifact: String,
        /// Disambiguate when several records are open
        #[arg(long)]
        raised: Option<String>,
    },
}

#[derive(Subcommand)]
enum InboxCmd {
    /// Answer an open question
    Answer {
        ticket: String,
        /// The answer, or the number of one of the options offered
        choice: String,
    },
}

#[derive(Subcommand)]
enum DispatchCmd {
    Scan {
        /// Override a tier's session: tier=model:effort:budget
        #[arg(long, value_name = "SPEC")]
        map: Vec<String>,
    },
}

#[derive(Subcommand)]
enum FmCmd {
    Get {
        file: PathBuf,
        field: String,
    },
    Set {
        file: PathBuf,
        field: String,
        value: String,
        /// Insert the field when absent
        #[arg(long)]
        create: bool,
    },
    /// Append to a list field (flow or block form)
    Add {
        file: PathBuf,
        field: String,
        value: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if matches!(cli.cmd, Cmd::Gate) {
        trellis::gate::run(); // never returns; always exits 0
    }
    match run(cli) {
        Ok(code) => code,
        Err(e) => {
            eprintln!("trellis: {e:#}");
            ExitCode::from(2)
        }
    }
}

fn spec_version(plugin_root: Option<&Path>) -> u32 {
    if let Some(root) = plugin_root {
        if let Ok(text) = std::fs::read_to_string(root.join("spec/model.md")) {
            if let Some(v) = text
                .lines()
                .next()
                .and_then(|t| t.split("(v").nth(1))
                .and_then(|r| r.split(')').next())
                .and_then(|d| d.parse().ok())
            {
                return v;
            }
        }
    }
    trellis::spec_version()
}

/// Normalize an artifact argument to a root-relative path.
fn to_rel(root: &Root, arg: &str) -> String {
    let p = Path::new(arg);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else {
        std::env::current_dir().unwrap_or_default().join(p)
    };
    if abs.exists() {
        let canon_abs = abs.canonicalize().unwrap_or(abs);
        let canon_root = root
            .path
            .canonicalize()
            .unwrap_or_else(|_| root.path.clone());
        if let Ok(rel) = canon_abs.strip_prefix(&canon_root) {
            return rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
        }
    }
    arg.trim_start_matches("./").to_string()
}

fn load(root_arg: Option<&Path>) -> anyhow::Result<(Tree, Git)> {
    let root = Root::discover(root_arg)?;
    if root.is_plugin_template() {
        anyhow::bail!(
            "{} is the plugin's own template, not a domain root",
            root.path.display()
        );
    }
    let git = Git::new(root.path.clone());
    let tree = Tree::load(root)?;
    Ok((tree, git))
}

/// A directory of the census, or an artifact in one. Children stay in the
/// order rows arrived, which is sorted by path.
enum Node {
    Dir(String, Vec<Node>),
    Artifact(String, usize),
}

fn insert(children: &mut Vec<Node>, segments: &[&str], row: usize) {
    let Some((head, rest)) = segments.split_first() else {
        return;
    };
    if rest.is_empty() {
        children.push(Node::Artifact(head.to_string(), row));
        return;
    }
    // Sorted input keeps a directory's entries contiguous, so the match — if
    // there is one — is the most recent child.
    if let Some(Node::Dir(name, sub)) = children.last_mut() {
        if name == head {
            insert(sub, rest, row);
            return;
        }
    }
    let mut sub = Vec::new();
    insert(&mut sub, rest, row);
    children.push(Node::Dir(head.to_string(), sub));
}

fn draw(children: &[Node], prefix: &str, out: &mut Vec<(String, Option<usize>)>) {
    for (i, child) in children.iter().enumerate() {
        let last = i + 1 == children.len();
        let (connector, carry) = if last {
            ("└── ", "    ")
        } else {
            ("├── ", "│   ")
        };
        match child {
            Node::Dir(name, sub) => {
                out.push((format!("{prefix}{connector}{name}/"), None));
                draw(sub, &format!("{prefix}{carry}"), out);
            }
            Node::Artifact(name, row) => {
                out.push((format!("{prefix}{connector}{name}"), Some(*row)));
            }
        }
    }
}

fn print_artifact_tree(report: &facts::TreeReport) {
    println!("{}", report.root);
    if report.artifacts.is_empty() {
        println!("(no artifacts match)");
    } else {
        let mut roots = Vec::new();
        for (i, a) in report.artifacts.iter().enumerate() {
            insert(&mut roots, &a.path.split('/').collect::<Vec<_>>(), i);
        }
        let mut lines = Vec::new();
        draw(&roots, "", &mut lines);

        // Pad so the annotations line up as columns rather than as drift.
        let width = lines
            .iter()
            .filter(|(_, row)| row.is_some())
            .map(|(text, _)| text.chars().count())
            .max()
            .unwrap_or(0);
        let kind_width = report
            .artifacts
            .iter()
            .map(|a| a.kind.len())
            .max()
            .unwrap_or(0);
        for (text, row) in &lines {
            let Some(a) = row.map(|r| &report.artifacts[r]) else {
                println!("{text}");
                continue;
            };
            let pad = " ".repeat(width.saturating_sub(text.chars().count()));
            let owner = a.owner.as_deref().unwrap_or("no owner");
            let status = a
                .status
                .as_deref()
                .map(|s| format!("  [{s}]"))
                .unwrap_or_default();
            println!(
                "{text}{pad}  {:<kind_width$}  {owner}{status}",
                a.kind,
                kind_width = kind_width
            );
        }
        println!("\n{} artifact(s)", report.artifacts.len());
    }
    print_scope(&report.scope);
}

/// What discovery left out. Silent when it left out nothing; never silent
/// when it did.
fn print_scope(scope: &trellis::tree::Scope) {
    if scope.is_empty() {
        return;
    }
    println!(
        "scope: {} git-ignored path(s), {} declared carried path(s), {} nested repo(s) excluded from artifact discovery",
        scope.git_ignored.len(),
        scope.carried.len(),
        scope.nested_repos.len()
    );
}

fn print_json<T: serde::Serialize>(value: &T) {
    println!(
        "{}",
        serde_json::to_string_pretty(value).expect("serializable")
    );
}

fn run(cli: Cli) -> anyhow::Result<ExitCode> {
    let ok = ExitCode::SUCCESS;
    let findings_exit = ExitCode::from(1);

    match cli.cmd {
        Cmd::Gate => unreachable!("handled in main"),

        Cmd::Root { quiet } => match Root::discover(cli.root.as_deref()) {
            Ok(root) => {
                if !quiet {
                    println!("{}", root.path.display());
                }
                Ok(ok)
            }
            Err(e) => {
                if !quiet {
                    eprintln!("trellis: {e}");
                }
                Ok(ExitCode::from(2))
            }
        },

        Cmd::Version => {
            println!(
                "trellis {} (spec v{})",
                env!("CARGO_PKG_VERSION"),
                spec_version(cli.plugin_root.as_deref())
            );
            Ok(ok)
        }

        Cmd::Lint {
            strict,
            items,
            paths,
        } => {
            let (tree, git) = load(cli.root.as_deref())?;
            let derived = graph::derive(&tree);
            let reg = registries::load(&tree);
            let ctx = lint::Ctx {
                tree: &tree,
                derived: &derived,
                git: &git,
                reg: &reg,
                today: dates::today(),
                spec_version: spec_version(cli.plugin_root.as_deref()),
            };
            let paths: Vec<String> = paths.iter().map(|p| to_rel(&tree.root, p)).collect();
            let report = lint::run(&ctx, items.as_deref(), &paths);
            match cli.format {
                Format::Json => print_json(&report),
                Format::Text => {
                    for f in &report.findings {
                        let line = f.line.map(|l| format!(":{l}")).unwrap_or_default();
                        let sev = match f.severity {
                            lint::Severity::Violation => "violation",
                            lint::Severity::Warning => "warning",
                        };
                        let owner = f
                            .owner
                            .as_deref()
                            .map(|o| format!(" → {o}"))
                            .unwrap_or_default();
                        println!(
                            "{}{line}: [item {}] {sev}: {}{owner}",
                            f.path, f.item, f.message
                        );
                    }
                    for j in &report.judgment {
                        println!("(judgment) item {}: {}", j.item, j.reason);
                    }
                    print_scope(&report.scope);
                    println!(
                        "{} violation(s), {} warning(s); {} item(s) run, {} with a judgment remainder",
                        report.summary.violations,
                        report.summary.warnings,
                        report.summary.items_run,
                        report.summary.items_judgment
                    );
                }
            }
            let failing = report.summary.violations > 0 || (strict && report.summary.warnings > 0);
            Ok(if failing { findings_exit } else { ok })
        }

        Cmd::Tree { kind, flat, paths } => {
            let (tree, _git) = load(cli.root.as_deref())?;
            let paths: Vec<String> = paths.iter().map(|p| to_rel(&tree.root, p)).collect();
            let mut artifacts = facts::artifact_rows(&tree);
            if !paths.is_empty() {
                artifacts.retain(|a| trellis::tree::under_any(&a.path, &paths));
            }
            if let Some(want) = &kind {
                artifacts.retain(|a| &a.kind == want);
            }
            let report = facts::TreeReport {
                version: 1,
                root: tree.root.path.display().to_string(),
                artifacts,
                scope: tree.scope.clone(),
            };
            match cli.format {
                Format::Json => print_json(&report),
                Format::Text if flat => {
                    for a in &report.artifacts {
                        println!("{}", a.path);
                    }
                }
                Format::Text => print_artifact_tree(&report),
            }
            Ok(ok)
        }

        Cmd::Readiness { plan } => {
            let (tree, _git) = load(cli.root.as_deref())?;
            let derived = graph::derive(&tree);
            let rel = to_rel(&tree.root, &plan);
            let report = readiness::check(&tree, &derived, &rel)?;
            match cli.format {
                Format::Json => print_json(&report),
                Format::Text => {
                    for i in &report.items {
                        let status = match i.status {
                            readiness::ItemStatus::Pass => "pass",
                            readiness::ItemStatus::Fail => "FAIL",
                            readiness::ItemStatus::Partial => "partial",
                            readiness::ItemStatus::Judgment => "judgment",
                        };
                        println!("[{:>2}] {status:<8} {} — {}", i.item, i.title, i.detail);
                    }
                    println!(
                        "mechanical share: {}",
                        if report.mechanical_pass {
                            "pass"
                        } else {
                            "FAIL"
                        }
                    );
                }
            }
            Ok(if report.mechanical_pass {
                ok
            } else {
                findings_exit
            })
        }

        Cmd::Show { artifact } => {
            let (tree, git) = load(cli.root.as_deref())?;
            let derived = graph::derive(&tree);
            let rel = to_rel(&tree.root, &artifact);
            let value = facts::artifact(&tree, &git, &derived, &rel, dates::today())
                .ok_or_else(|| anyhow::anyhow!("{rel} is not an artifact in this root"))?;
            match cli.format {
                Format::Json => print_json(&value),
                Format::Text => {
                    let obj = value
                        .as_object()
                        .expect("facts::artifact returns an object");
                    for (k, v) in obj {
                        match v {
                            serde_json::Value::Null => {}
                            serde_json::Value::String(s) => println!("{k}: {s}"),
                            other => println!("{k}: {other}"),
                        }
                    }
                }
            }
            Ok(ok)
        }

        Cmd::Query { what, artifact } => {
            let (tree, git) = load(cli.root.as_deref())?;
            let derived = graph::derive(&tree);
            let rel = to_rel(&tree.root, &artifact);
            let a = tree
                .get(&rel)
                .ok_or_else(|| anyhow::anyhow!("{rel} is not an artifact in this root"))?;
            match what {
                QueryWhat::Status => println!(
                    "{}",
                    a.status()
                        .ok_or_else(|| anyhow::anyhow!("{rel} declares no status:"))?
                ),
                QueryWhat::Band => println!(
                    "{}",
                    derived
                        .band(&rel)
                        .map(|b| b.as_str())
                        .ok_or_else(|| anyhow::anyhow!("{rel} has no strategy band"))?
                ),
                QueryWhat::Class => {
                    let class = match a.kind {
                        Kind::Problem => Some(derived.effective_class(&rel)),
                        Kind::Plan => derived.plan_class(&tree, &rel),
                        _ => None,
                    };
                    println!(
                        "{}",
                        class
                            .map(|c| c.as_str())
                            .ok_or_else(|| anyhow::anyhow!("{rel} has no effective class"))?
                    );
                }
                QueryWhat::Dwell => {
                    let status = a
                        .status()
                        .ok_or_else(|| anyhow::anyhow!("{rel} declares no status:"))?;
                    match git.status_set_date(&rel, &status) {
                        Some(set) => println!("{}", dates::days_between(set, dates::today())),
                        None => println!("0"),
                    }
                }
                QueryWhat::Held => match derived.hold(&rel) {
                    Some((target, status)) => println!(
                        "held — awaits {target} (status: {})",
                        status
                            .map(|s| s.as_str().to_string())
                            .unwrap_or_else(|| "missing".into())
                    ),
                    None => println!("not held"),
                },
            }
            Ok(ok)
        }

        Cmd::Resolve { r#ref } => {
            let (tree, _git) = load(cli.root.as_deref())?;
            match refs::resolve(&r#ref, &tree) {
                Ok(()) => {
                    match refs::classify(&r#ref) {
                        refs::RefKind::Path {
                            path,
                            anchor: Some(anchor),
                        } => {
                            let line = tree
                                .get(&path)
                                .and_then(|a| {
                                    let want = trellis::markdown::slugify(&anchor);
                                    a.headings.iter().find(|h| h.slug == want).map(|h| h.line)
                                })
                                .unwrap_or(0);
                            println!("{path}:{line}");
                        }
                        refs::RefKind::Path { path, anchor: None } => println!("{path}"),
                        refs::RefKind::Role(r) => {
                            println!("{}/mandate.md", r)
                        }
                        refs::RefKind::SelfFunding => println!("self"),
                        refs::RefKind::External(e) => println!("external: {e}"),
                        refs::RefKind::Glob(g) => println!("glob: {g}"),
                    }
                    Ok(ok)
                }
                Err(reason) => {
                    eprintln!("unresolved: {reason}");
                    Ok(findings_exit)
                }
            }
        }

        Cmd::Archive {
            artifact,
            sweep,
            dry_run,
        } => archive_cmd(cli.root.as_deref(), artifact.as_deref(), sweep, dry_run),

        Cmd::Plan { cmd } => plan_cmd(cli.root.as_deref(), cli.format, cmd),
        Cmd::Escalate { cmd } => escalate_cmd(cli.root.as_deref(), cli.format, cmd),

        Cmd::Inbox { cmd, watch, addr } => {
            let root = Root::discover(cli.root.as_deref())?.path;
            let addr = addr.as_deref();
            match cmd {
                Some(InboxCmd::Answer { ticket, choice }) => {
                    let resolved = daemon::client::answer(&root, addr, &ticket, &choice)?;
                    match cli.format {
                        Format::Json => print_json(&serde_json::json!({
                            "ticket": ticket, "answer": resolved,
                        })),
                        Format::Text => println!("answered {ticket}: {resolved}"),
                    }
                    Ok(ExitCode::SUCCESS)
                }
                None => {
                    let open = if watch {
                        daemon::client::watch(&root, addr)?
                    } else {
                        daemon::client::pending(&root, addr)?
                    };
                    match cli.format {
                        Format::Json => print_json(&open),
                        Format::Text => println!("{}", daemon::client::render(&open)),
                    }
                    Ok(ExitCode::SUCCESS)
                }
            }
        }

        Cmd::Dispatch {
            cmd: DispatchCmd::Scan { map },
        } => {
            let (tree, _git) = load(cli.root.as_deref())?;
            let mut sessions = SessionMap::default();
            for spec in &map {
                sessions.apply(spec)?;
            }
            let report = dispatch::scan(&tree, &sessions);
            match cli.format {
                Format::Json => print_json(&report),
                Format::Text => {
                    for w in &report.warnings {
                        println!("warning: {w}");
                    }
                    for h in &report.held {
                        println!(
                            "held: {} — awaits {} (status: {})",
                            h.plan,
                            h.awaits,
                            h.target_status.as_deref().unwrap_or("missing")
                        );
                    }
                    for d in &report.dispatch {
                        println!(
                            "dispatch: {} → {} (complexity: {}, {} / {} / ${})",
                            d.plan, d.owner, d.complexity, d.model, d.effort, d.budget_usd
                        );
                    }
                    if report.dispatch.is_empty() && report.held.is_empty() {
                        println!("(no ready plans)");
                    }
                }
            }
            Ok(ok)
        }

        Cmd::View {
            name,
            write,
            out,
            check,
        } => {
            let (tree, git) = load(cli.root.as_deref())?;
            let today = dates::today();
            let rendered = views::render_named(&name, &tree, &git, today).ok_or_else(|| {
                anyhow::anyhow!("{name} is not a view ({})", views::NAMES.join(" | "))
            })?;
            let dest: Option<PathBuf> =
                out.or_else(|| views::default_path(&name).map(|p| tree.root.path.join(p)));
            if check {
                let dest = dest.ok_or_else(|| {
                    anyhow::anyhow!(
                        "view {name} has no canonical path — pass --out to check a written copy"
                    )
                })?;
                let existing = std::fs::read_to_string(&dest).unwrap_or_default();
                let normalize = |s: &str| -> String {
                    s.lines()
                        .filter(|l| !l.starts_with("date:"))
                        .collect::<Vec<_>>()
                        .join("\n")
                };
                if normalize(&existing) == normalize(&rendered) {
                    println!("{}: fresh", dest.display());
                    Ok(ok)
                } else {
                    println!(
                        "{}: differs from `trellis view {name}` output — regenerate with --write",
                        dest.display()
                    );
                    Ok(findings_exit)
                }
            } else if write {
                let dest = dest.ok_or_else(|| {
                    anyhow::anyhow!("view {name} has no canonical path — pass --out (register one in conventions.md)")
                })?;
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::write(&dest, &rendered)?;
                println!("{}", dest.display());
                Ok(ok)
            } else {
                print!("{rendered}");
                Ok(ok)
            }
        }

        Cmd::Fm { cmd } => match cmd {
            FmCmd::Get { file, field } => match fmedit::get(&file, &field)? {
                Some(v) => {
                    println!("{v}");
                    Ok(ok)
                }
                None => Ok(findings_exit),
            },
            FmCmd::Set {
                file,
                field,
                value,
                create,
            } => {
                fmedit::set_scalar(&file, &field, &value, create)?;
                Ok(ok)
            }
            FmCmd::Add { file, field, value } => {
                fmedit::append_list(&file, &field, &value)?;
                Ok(ok)
            }
        },

        Cmd::Scaffold { dir, owner } => {
            let written =
                scaffold::scaffold(&dir, &owner, dates::today(), cli.plugin_root.as_deref())?;
            println!("scaffolded {} files into {}", written.len(), dir.display());
            Ok(ok)
        }

        Cmd::Serve {
            port,
            bind,
            config,
            once,
            dry_run,
            no_http,
            tick_secs,
            max_concurrent,
            map,
        } => daemon::run(daemon::ServeOpts {
            root: cli.root,
            plugin_root: cli.plugin_root,
            config,
            bind,
            port,
            tick_secs,
            max_concurrent,
            map,
            once,
            dry_run,
            no_http,
        }),
    }
}

/// `trellis archive` — the move that follows a terminal verdict, never the
/// verdict itself. Retirement stays a frontmatter flip in place so the
/// closure event lands under the live path where the board's flow reading
/// sees it; this files the artifact away afterwards.
fn archive_cmd(
    root_arg: Option<&Path>,
    artifact: Option<&str>,
    sweep: bool,
    dry_run: bool,
) -> anyhow::Result<ExitCode> {
    let (tree, git) = load(root_arg)?;
    if !git.is_repo() {
        anyhow::bail!(
            "not a git repository — archiving is a `git mv` so that history follows the artifact"
        );
    }
    let reg = registries::load(&tree);
    let today = dates::today();

    // (path, why) for everything that will move.
    let mut moves: Vec<(String, String)> = Vec::new();

    if let Some(arg) = artifact {
        let rel = to_rel(&tree.root, arg);
        let Some(a) = tree.get(&rel) else {
            anyhow::bail!("{rel} is not an artifact in this root");
        };
        if a.archived {
            println!("{rel}: already filed");
            return Ok(ExitCode::SUCCESS);
        }
        let Some(want) = trellis::tree::terminal_status(a.kind, &a.rel) else {
            anyhow::bail!(
                "{rel} has no terminal status of its own — it is archived with the subtree it belongs to"
            );
        };
        match a.status() {
            Some(s) if s == want => moves.push((rel, format!("status: {want}"))),
            other => anyhow::bail!(
                "{rel} declares status: {} — the tier admits terminal artifacts only, which for this kind means status: {want}",
                other.as_deref().unwrap_or("(none)")
            ),
        }
    } else if sweep {
        let Some(horizon) = reg.archive_after_days else {
            anyhow::bail!(
                "no retention horizon declared — add an \"archive after N days\" statement to conventions.md, or name an artifact explicitly"
            );
        };
        for a in &tree.artifacts {
            if a.archived {
                continue;
            }
            let Some(want) = trellis::tree::terminal_status(a.kind, &a.rel) else {
                continue;
            };
            if a.status().as_deref() != Some(want) {
                continue;
            }
            // Cold means the terminal status has held for the horizon. An
            // uncommitted flip has no dwell yet and is never swept.
            let Some(set) = git.status_set_date(&a.rel, want) else {
                continue;
            };
            let age = dates::days_between(set, today);
            if age >= horizon {
                moves.push((a.rel.clone(), format!("{want} for {age}d")));
            }
        }
    } else {
        anyhow::bail!("name an artifact to file, or pass --sweep");
    }

    moves.sort();
    if moves.is_empty() {
        println!("nothing to file");
        return Ok(ExitCode::SUCCESS);
    }
    for (rel, why) in &moves {
        let dest = format!("{}{rel}", trellis::tree::ARCHIVE);
        if dry_run {
            println!("would file {rel} → {dest} ({why})");
        } else {
            git.mv(rel, &dest)?;
            println!("{rel} → {dest} ({why})");
        }
    }
    Ok(ExitCode::SUCCESS)
}

fn plan_cmd(root_arg: Option<&Path>, format: Format, cmd: PlanCmd) -> anyhow::Result<ExitCode> {
    let ok = ExitCode::SUCCESS;
    let (tree, git) = load(root_arg)?;
    let derived = graph::derive(&tree);

    match cmd {
        PlanCmd::List {
            status,
            held,
            archived,
        } => {
            let rows: Vec<facts::PlanRow> = facts::plan_rows(&tree, &git, &derived, dates::today())
                .into_iter()
                // The default census is the live one — the whole reason the
                // tier exists. `--archived` asks for the rest.
                .filter(|r| archived || !r.archived)
                .filter(|r| !held || r.held.is_some())
                .filter(|r| match &status {
                    Some(want) => r.status.as_deref() == Some(want.as_str()),
                    None => true,
                })
                .collect();
            match format {
                Format::Json => print_json(&rows),
                Format::Text => {
                    for r in &rows {
                        let held = r
                            .held
                            .as_deref()
                            .map(|t| format!(" [held — awaits {t}]"))
                            .unwrap_or_default();
                        println!(
                            "{} — {} ({}, {}){}{}",
                            r.plan,
                            r.status.as_deref().unwrap_or("no status"),
                            r.r#type.as_deref().unwrap_or("no type"),
                            r.owner.as_deref().unwrap_or("no owner"),
                            r.dwell_days.map(|d| format!(" {d}d")).unwrap_or_default(),
                            held,
                        );
                    }
                    if rows.is_empty() {
                        println!("(no plans match)");
                    }
                }
            }
            Ok(ok)
        }

        PlanCmd::Release { plan, force } => {
            let rel = to_rel(&tree.root, &plan);
            if !force {
                let report = readiness::check(&tree, &derived, &rel)?;
                if !report.mechanical_pass {
                    for i in report
                        .items
                        .iter()
                        .filter(|i| i.status == readiness::ItemStatus::Fail)
                    {
                        eprintln!("readiness [{:>2}] FAIL {} — {}", i.item, i.title, i.detail);
                    }
                    anyhow::bail!(
                        "{rel} fails the mechanical readiness share — it stays draft (walk checks/plan-readiness.md, or --force past the gate)"
                    );
                }
            }
            let abs = tree.root.path.join(&rel);
            plan_ops::flip(&abs, &[PlanStatus::Draft], PlanStatus::Ready)?;
            println!("{rel}: draft → ready (judgment readiness items stay the owner's call)");
            Ok(ok)
        }

        PlanCmd::Claim { plan } => {
            let rel = to_rel(&tree.root, &plan);
            if let Some((target, status)) = derived.hold(&rel) {
                anyhow::bail!(
                    "{rel} is held — awaits {target} (status: {}); the hold clears when every target retires",
                    status.map(|s| s.as_str().to_string()).unwrap_or_else(|| "missing".into())
                );
            }
            let abs = tree.root.path.join(&rel);
            plan_ops::flip(&abs, &[PlanStatus::Ready], PlanStatus::Active)?;
            println!("{rel}: ready → active");
            Ok(ok)
        }

        PlanCmd::Block {
            plan,
            asks,
            attempted,
            blocked,
            by,
        } => {
            let rel = to_rel(&tree.root, &plan);
            let artifact = tree
                .get(&rel)
                .ok_or_else(|| anyhow::anyhow!("{rel} is not an artifact in this root"))?;
            let by = match by {
                Some(b) => b,
                None => artifact
                    .owner()
                    .ok_or_else(|| anyhow::anyhow!("{rel} has no owner: — pass --by"))?,
            };
            let to = escalate::add(
                &tree,
                &rel,
                &NewEscalation {
                    by,
                    to: None,
                    asks,
                    attempted,
                    blocked,
                },
                dates::today(),
            )?;
            let abs = tree.root.path.join(&rel);
            plan_ops::flip(
                &abs,
                &[PlanStatus::Ready, PlanStatus::Active],
                PlanStatus::Blocked,
            )?;
            println!("{rel}: → blocked, open escalation recorded (to: {to})");
            Ok(ok)
        }

        PlanCmd::Unblock { plan } => {
            let rel = to_rel(&tree.root, &plan);
            let abs = tree.root.path.join(&rel);
            plan_ops::flip(&abs, &[PlanStatus::Blocked], PlanStatus::Ready)?;
            let open = tree
                .get(&rel)
                .map(|a| {
                    trellis::model::escalation_records(a)
                        .iter()
                        .filter(|r| r.status.as_deref() == Some("open"))
                        .count()
                })
                .unwrap_or(0);
            println!("{rel}: blocked → ready");
            if open > 0 {
                println!(
                    "note: {open} open escalation record(s) remain — resolution is the owner's edit (trellis escalate resolve)"
                );
            }
            Ok(ok)
        }

        PlanCmd::Retire { plan } => {
            let rel = to_rel(&tree.root, &plan);
            let abs = tree.root.path.join(&rel);
            let from = [
                PlanStatus::Draft,
                PlanStatus::Ready,
                PlanStatus::Active,
                PlanStatus::Blocked,
            ];
            plan_ops::flip(&abs, &from, PlanStatus::Retired)?;
            // `awaits:` edges name the live path, whatever tier the target
            // sits in.
            let address = trellis::tree::live_path(&rel).to_string();
            let dependents: Vec<String> = derived
                .plan_awaits
                .iter()
                .filter(|(_, targets)| targets.contains(&address))
                .map(|(p, _)| p.clone())
                .collect();
            println!("{rel}: → retired");
            if !dependents.is_empty() {
                let mut d = dependents;
                d.sort();
                println!("awaits dependents that may now clear: {}", d.join(", "));
            }
            Ok(ok)
        }
    }
}

fn escalate_cmd(
    root_arg: Option<&Path>,
    format: Format,
    cmd: EscalateCmd,
) -> anyhow::Result<ExitCode> {
    let ok = ExitCode::SUCCESS;
    let (tree, _git) = load(root_arg)?;
    match cmd {
        EscalateCmd::Add {
            artifact,
            by,
            to,
            asks,
            attempted,
            blocked,
        } => {
            let rel = to_rel(&tree.root, &artifact);
            let to = escalate::add(
                &tree,
                &rel,
                &NewEscalation {
                    by,
                    to,
                    asks,
                    attempted,
                    blocked,
                },
                dates::today(),
            )?;
            println!("{rel}: open escalation recorded (to: {to})");
            Ok(ok)
        }
        EscalateCmd::List { all } => {
            let records = escalate::list(&tree, all);
            match format {
                Format::Json => print_json(&records),
                Format::Text => {
                    for r in &records {
                        println!(
                            "{}:{} [{}] raised {} by {} to {} — {}",
                            r.artifact,
                            r.line,
                            r.status.as_deref().unwrap_or("?"),
                            r.raised.as_deref().unwrap_or("?"),
                            r.by.as_deref().unwrap_or("?"),
                            r.to.as_deref().unwrap_or("?"),
                            r.asks.as_deref().unwrap_or("—"),
                        );
                    }
                    if records.is_empty() {
                        println!("(no {}records)", if all { "" } else { "open " });
                    }
                }
            }
            Ok(ok)
        }
        EscalateCmd::Resolve { artifact, raised } => {
            let rel = to_rel(&tree.root, &artifact);
            let record = escalate::resolve(&tree, &rel, raised.as_deref())?;
            println!(
                "{rel}: record raised {} flipped open → resolved — answer in prose beneath it; any status flip it unblocks is a separate change",
                record.raised.as_deref().unwrap_or("?")
            );
            Ok(ok)
        }
    }
}
