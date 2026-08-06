//! The mechanical share of `checks/conventions-lint.md`, item-for-item.
//! That file remains the single source; this engine mirrors it and reports
//! what it cannot judge. Items 7 and 16 are judgment by design; items 2 and
//! 23 run their mechanical halves and report the remainder.

use serde::Serialize;

use crate::dates::{self, Date};
use crate::gitio::Git;
use crate::graph::Derived;
use crate::model::escalation_records;
use crate::model::{Band, Class, Complexity, PlanStatus, Provenance, StrategyStatus};
use crate::refs;
use crate::registries::Registries;
use crate::tree::{Kind, Scope, Tree};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Violation,
    Warning,
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub item: u8,
    pub severity: Severity,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<u32>,
    /// The artifact's owner — each failure becomes an escalation to them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct JudgmentNote {
    pub item: u8,
    pub reason: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub checked_mechanically: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Summary {
    pub violations: usize,
    pub warnings: usize,
    pub items_run: usize,
    pub items_judgment: usize,
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub version: u32,
    pub root: String,
    pub spec_version: u32,
    pub findings: Vec<Finding>,
    pub judgment: Vec<JudgmentNote>,
    pub summary: Summary,
    /// What the walk was not allowed to see. Carried here because a narrowing
    /// nobody reports reads as "everything was checked".
    pub scope: Scope,
}

pub struct Ctx<'a> {
    pub tree: &'a Tree,
    pub derived: &'a Derived,
    pub git: &'a Git,
    pub reg: &'a Registries,
    pub today: Date,
    pub spec_version: u32,
}

struct Sink {
    findings: Vec<Finding>,
    judgment: Vec<JudgmentNote>,
}

impl Sink {
    fn push(
        &mut self,
        ctx: &Ctx,
        item: u8,
        severity: Severity,
        path: &str,
        line: Option<u32>,
        message: String,
    ) {
        let owner = ctx.tree.get(path).and_then(|a| a.owner());
        self.findings.push(Finding {
            item,
            severity,
            path: path.to_string(),
            line,
            owner,
            message,
        });
    }

    fn violation(&mut self, ctx: &Ctx, item: u8, path: &str, line: Option<u32>, message: String) {
        self.push(ctx, item, Severity::Violation, path, line, message);
    }

    fn warning(&mut self, ctx: &Ctx, item: u8, path: &str, line: Option<u32>, message: String) {
        self.push(ctx, item, Severity::Warning, path, line, message);
    }

    fn judgment(&mut self, item: u8, reason: &str, checked: Vec<String>) {
        self.judgment.push(JudgmentNote {
            item,
            reason: reason.to_string(),
            checked_mechanically: checked,
        });
    }
}

type RuleFn = fn(&Ctx, &mut Sink);

/// Items with no mechanical component at all.
const JUDGMENT_ONLY: &[u8] = &[7, 16];

/// One entry per item in `checks/conventions-lint.md`, which stays the single
/// source. The lockstep test (`tests/lockstep.rs`) asserts this table covers
/// exactly that checklist's items — an item added to the prose without a rule
/// here fails CI rather than going quietly unchecked (decision 0037).
static RULES: &[(u8, RuleFn)] = &[
    (1, item01),
    (2, item02),
    (3, item03),
    (4, item04),
    (5, item05),
    (6, item06),
    (7, item07),
    (8, item08),
    (9, item09),
    (10, item10),
    (11, item11),
    (12, item12),
    (13, item13),
    (14, item14),
    (15, item15),
    (16, item16),
    (17, item17),
    (18, item18),
    (19, item19),
    (20, item20),
    (21, item21),
    (22, item22),
    (23, item23),
    (24, item24),
    (25, item25),
];

/// The checklist items this kernel implements, in order.
pub fn implemented_items() -> Vec<u8> {
    RULES.iter().map(|(i, _)| *i).collect()
}

pub fn run(ctx: &Ctx, items: Option<&[u8]>, paths: &[String]) -> Report {
    let mut sink = Sink {
        findings: vec![],
        judgment: vec![],
    };
    let want = |i: u8| items.map(|list| list.contains(&i)).unwrap_or(true);

    for (i, rule) in RULES {
        if want(*i) {
            rule(ctx, &mut sink);
        }
    }

    if !paths.is_empty() {
        sink.findings
            .retain(|f| crate::tree::under_any(&f.path, paths));
    }

    sink.findings
        .sort_by(|a, b| (a.path.as_str(), a.item, a.line).cmp(&(b.path.as_str(), b.item, b.line)));

    let violations = sink
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Violation)
        .count();
    let warnings = sink.findings.len() - violations;
    let items_considered = items.map(|l| l.len()).unwrap_or(RULES.len());
    Report {
        version: 1,
        root: ctx.tree.root.path.display().to_string(),
        spec_version: ctx.spec_version,
        summary: Summary {
            violations,
            warnings,
            items_run: items_considered
                - sink
                    .judgment
                    .iter()
                    .filter(|j| JUDGMENT_ONLY.contains(&j.item))
                    .count(),
            items_judgment: sink.judgment.len(),
        },
        findings: sink.findings,
        judgment: sink.judgment,
        scope: ctx.tree.scope.clone(),
    }
}

// 1. Every artifact has frontmatter with valid `provenance:` and `owner:`.
fn item01(ctx: &Ctx, sink: &mut Sink) {
    for a in &ctx.tree.artifacts {
        let Some(fm) = &a.fm else {
            sink.violation(
                ctx,
                1,
                &a.rel,
                None,
                "artifact has no frontmatter — every artifact declares provenance: and owner:"
                    .into(),
            );
            continue;
        };
        match fm.get_str("provenance") {
            None => sink.violation(
                ctx,
                1,
                &a.rel,
                Some(1),
                "frontmatter declares no provenance: (authored | generated | state-ref)".into(),
            ),
            Some(p) if Provenance::parse(&p).is_none() => sink.violation(
                ctx,
                1,
                &a.rel,
                fm.line_of("provenance"),
                format!("provenance: {p} is not a legal class (authored | generated | state-ref)"),
            ),
            _ => {}
        }
        match fm.get_str("owner") {
            None => sink.violation(
                ctx,
                1,
                &a.rel,
                Some(1),
                "frontmatter declares no owner: (org/{role})".into(),
            ),
            Some(o) => {
                if !ctx.tree.role_exists(&o) {
                    sink.violation(
                        ctx, 1, &a.rel, fm.line_of("owner"),
                        format!("owner: {o} does not resolve to a role in this root (no {o}/mandate.md)"),
                    );
                }
            }
        }
    }
}

// 2. No `generated` artifact has hand-edits — mechanical only for views this
// kernel generates (regenerate and diff); judgment for the rest.
fn item02(ctx: &Ctx, sink: &mut Sink) {
    let mut checked = Vec::new();
    for a in &ctx.tree.artifacts {
        if a.provenance().as_deref() != Some("generated") {
            continue;
        }
        let Some(name) =
            a.fm.as_ref()
                .and_then(|fm| fm.get_str("generated-by"))
                .and_then(|g| g.strip_prefix("trellis view ").map(str::to_string))
        else {
            continue;
        };
        let Some(fresh) = crate::views::render_named(&name, ctx.tree, ctx.git, ctx.today) else {
            continue;
        };
        let normalize = |s: &str| -> String {
            s.lines()
                .filter(|l| !l.starts_with("date:"))
                .collect::<Vec<_>>()
                .join("\n")
        };
        checked.push(a.rel.clone());
        if normalize(&a.text) != normalize(&fresh) {
            sink.violation(
                ctx, 2, &a.rel, None,
                format!("differs from its generator's output (`trellis view {name}`) — hand-edited or stale; regenerate with `trellis view {name} --write`"),
            );
        }
    }
    sink.judgment(
        2,
        "generated artifacts without a `generated-by: trellis view …` stamp need the suspicious-edit judgment call",
        checked,
    );
}

// 3. Every subdomain declares >=1 induced-by edge; edges name an existing
// strategy and a legal class; no node-level class:.
fn item03(ctx: &Ctx, sink: &mut Sink) {
    for a in ctx.tree.by_kind(Kind::Problem) {
        let Some(fm) = &a.fm else { continue };
        let line = fm.line_of("induced-by");
        let edges = fm.get_edges("induced-by", "strategy").unwrap_or_default();
        if edges.is_empty() {
            sink.violation(ctx, 3, &a.rel, Some(1), "declares no induced-by: edge — every subdomain names the strategy that put it in the domain".into());
        }
        for edge in &edges {
            if edge.target.is_empty() {
                sink.violation(
                    ctx,
                    3,
                    &a.rel,
                    line,
                    "induced-by: edge names no strategy".into(),
                );
            } else if !edge.target.starts_with("strategy/") || ctx.tree.get(&edge.target).is_none()
            {
                sink.violation(ctx, 3, &a.rel, line, format!("induced-by: edge names {}, which is not an existing strategy/{{strategy}}.md", edge.target));
            }
            match &edge.class {
                None => sink.violation(
                    ctx,
                    3,
                    &a.rel,
                    line,
                    format!(
                        "induced-by: edge to {} declares no class: (core | supporting | generic)",
                        edge.target
                    ),
                ),
                Some(c) if Class::parse(c).is_none() => sink.violation(
                    ctx,
                    3,
                    &a.rel,
                    line,
                    format!("class: {c} is not legal (core | supporting | generic)"),
                ),
                _ => {}
            }
        }
        if fm.has_key("class") {
            sink.violation(
                ctx,
                3,
                &a.rel,
                fm.line_of("class"),
                "node-level class: remains — classification is a property of the induced-by edge"
                    .into(),
            );
        }
    }
}

// 4. Plan frontmatter: legal status, registered type, resolving refs, legal
// complexity, resolving awaits (never self, never outside plans/).
fn item04(ctx: &Ctx, sink: &mut Sink) {
    for a in ctx.tree.by_kind(Kind::Plan) {
        let Some(fm) = &a.fm else { continue };
        match fm.get_str("status") {
            None => sink.violation(
                ctx,
                4,
                &a.rel,
                Some(1),
                "plan declares no status: (draft | ready | active | blocked | retired)".into(),
            ),
            Some(s) if PlanStatus::parse(&s).is_none() => sink.violation(
                ctx,
                4,
                &a.rel,
                fm.line_of("status"),
                format!("status: {s} is not legal (draft | ready | active | blocked | retired)"),
            ),
            _ => {}
        }
        match fm.get_str("type") {
            None => sink.violation(ctx, 4, &a.rel, Some(1), "plan declares no type: — register one in conventions.md's plan-type registry first".into()),
            Some(t) if !ctx.reg.plan_types.contains(&t) => sink.violation(ctx, 4, &a.rel, fm.line_of("type"), format!("type: {t} is not registered in conventions.md's plan-type registry")),
            _ => {}
        }
        if let Some(c) = fm.get_str("complexity") {
            if Complexity::parse(&c).is_none() {
                sink.violation(
                    ctx,
                    4,
                    &a.rel,
                    fm.line_of("complexity"),
                    format!("complexity: {c} is not a legal tier (mechanical | standard | deep)"),
                );
            }
        }
        for key in ["subdomains", "contexts", "metrics", "decisions"] {
            for r in fm.get_list(key).unwrap_or_default() {
                if let Err(reason) = refs::resolve(&r, ctx.tree) {
                    sink.violation(
                        ctx,
                        4,
                        &a.rel,
                        fm.line_of(key),
                        format!("{key}: ref {r} does not resolve — {reason}"),
                    );
                }
            }
        }
        for target in fm.get_list("awaits").unwrap_or_default() {
            if target == a.rel {
                sink.violation(
                    ctx,
                    4,
                    &a.rel,
                    fm.line_of("awaits"),
                    "awaits: names the plan itself".into(),
                );
            } else if !target.starts_with("plans/") {
                // The tier is never an address: `archive/plans/x.md` fails
                // here on purpose, and item 25 says why.
                sink.violation(ctx, 4, &a.rel, fm.line_of("awaits"), format!("awaits: target {target} is outside plans/ — sequencing edges only point at plans in this root"));
            } else if ctx.tree.get_addressed(&target).is_none() {
                sink.violation(ctx, 4, &a.rel, fm.line_of("awaits"), format!("awaits: target {target} does not resolve — dispatch holds this plan, fail closed"));
            }
        }
    }
}

// 5. Every mandate declares authority: and escalate-to:; every role has a
// holder (package or ref).
fn item05(ctx: &Ctx, sink: &mut Sink) {
    for a in ctx.tree.by_kind(Kind::Mandate) {
        let Some(fm) = &a.fm else { continue };
        if !fm.has_key("authority") {
            sink.violation(
                ctx,
                5,
                &a.rel,
                Some(1),
                "mandate declares no authority: (spend, publish, approve)".into(),
            );
        }
        match fm.get_str("escalate-to") {
            None => sink.violation(
                ctx,
                5,
                &a.rel,
                Some(1),
                "mandate declares no escalate-to:".into(),
            ),
            Some(to) => {
                if !ctx.tree.role_exists(&to) {
                    sink.violation(
                        ctx,
                        5,
                        &a.rel,
                        fm.line_of("escalate-to"),
                        format!("escalate-to: {to} does not resolve to a role in this root"),
                    );
                }
            }
        }
        let role_dir = a.rel.trim_end_matches("/mandate.md");
        let holder_dir = format!("{role_dir}/holder");
        let has_package = ctx.tree.get(&format!("{holder_dir}/system.md")).is_some();
        let has_ref = ctx.tree.get(&format!("{holder_dir}/ref.md")).is_some();
        let external = fm
            .get_str("holder")
            .map(|h| h != "holder/" && !h.is_empty())
            .unwrap_or(false);
        if !has_package && !has_ref && !external {
            sink.violation(ctx, 5, &a.rel, fm.line_of("holder"), format!("role has no holder — add {holder_dir}/system.md (package), {holder_dir}/ref.md, or an external ref in holder:"));
        }
    }
}

// 6. Every agent holder package has at least one eval (ref.md exempt).
fn item06(ctx: &Ctx, sink: &mut Sink) {
    for role in ctx.tree.roles() {
        let holder = format!("org/{role}/holder");
        let is_package = ctx.tree.get(&format!("{holder}/system.md")).is_some();
        let is_ref = ctx.tree.get(&format!("{holder}/ref.md")).is_some();
        if !is_package || is_ref {
            continue;
        }
        let evals = format!("{holder}/evals/");
        let has_eval = ctx
            .tree
            .artifacts
            .iter()
            .map(|a| &a.rel)
            .chain(ctx.tree.other_files.iter())
            .any(|f| f.starts_with(&evals));
        if !has_eval {
            sink.violation(
                ctx,
                6,
                &format!("{holder}/system.md"),
                None,
                format!("agent holder package has no eval — add at least one under {evals}"),
            );
        }
    }
}

fn item07(_ctx: &Ctx, sink: &mut Sink) {
    sink.judgment(
        7,
        "terms-of-art coverage against glossary.md is a judgment check",
        vec![],
    );
}

// 8. Tags in use are registered in conventions.md.
fn item08(ctx: &Ctx, sink: &mut Sink) {
    for a in &ctx.tree.artifacts {
        let Some(fm) = &a.fm else { continue };
        for tag in fm.get_list("tags").unwrap_or_default() {
            if !ctx.reg.tags.contains(&tag) {
                sink.violation(ctx, 8, &a.rel, fm.line_of("tags"), format!("tag {tag} is not registered in conventions.md's tag registry — register before use"));
            }
        }
    }
}

// 9. decisions/ append-only: no accepted decision has been edited.
fn item09(ctx: &Ctx, sink: &mut Sink) {
    if !ctx.git.is_repo() {
        return;
    }
    let re = regex::Regex::new(r"^decisions/\d{4}-[^/]+\.md$").unwrap();
    for a in ctx.tree.by_kind(Kind::Decision) {
        if !re.is_match(&a.rel) {
            continue;
        }
        let commits = ctx.git.commits(&a.rel);
        if commits.is_empty() {
            continue; // uncommitted draft — still part of the session
        }
        let accepted_at = commits.iter().position(|t| {
            ctx.git
                .text_at(&t.sha, &t.path)
                .and_then(|t| crate::frontmatter::extract(&t))
                .and_then(|fm| fm.get_str("status"))
                .as_deref()
                == Some("accepted")
        });
        let Some(idx) = accepted_at else { continue };
        if idx < commits.len() - 1 {
            sink.violation(ctx, 9, &a.rel, None, "accepted decision was edited after acceptance — decisions/ is append-only; supersede with a new numbered decision".into());
        }
        if ctx.git.is_dirty(&a.rel) {
            sink.violation(ctx, 9, &a.rel, None, "accepted decision has uncommitted edits — decisions/ is append-only; supersede with a new numbered decision".into());
        }
    }
}

// 10. No secrets anywhere in the root.
fn item10(ctx: &Ctx, sink: &mut Sink) {
    let hard: &[(&str, &str)] = &[
        (
            r"-----BEGIN (?:RSA |EC |DSA |OPENSSH |PGP )?PRIVATE KEY",
            "private key material",
        ),
        (r"\bAKIA[0-9A-Z]{16}\b", "AWS access key id"),
        (r"\bghp_[A-Za-z0-9]{36,}\b", "GitHub personal access token"),
        (
            r"\bgithub_pat_[A-Za-z0-9_]{22,}\b",
            "GitHub fine-grained token",
        ),
        (r"\bgho_[A-Za-z0-9]{36,}\b", "GitHub OAuth token"),
        (r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b", "Slack token"),
        (r"\bsk-ant-[A-Za-z0-9_-]{20,}\b", "Anthropic API key"),
        (r"\bAIza[0-9A-Za-z_-]{35}\b", "Google API key"),
    ];
    let soft = regex::Regex::new(
        r#"(?i)\b(api[_-]?key|secret|password|access[_-]?token)\b\s*[:=]\s*["']?[A-Za-z0-9+/_\-]{24,}"#,
    )
    .unwrap();
    let hard: Vec<(regex::Regex, &str)> = hard
        .iter()
        .map(|(p, label)| (regex::Regex::new(p).unwrap(), *label))
        .collect();

    let scan = |rel: &str, text: &str, sink: &mut Sink| {
        for (i, line) in text.lines().enumerate() {
            for (re, label) in &hard {
                if re.is_match(line) {
                    sink.violation(ctx, 10, rel, Some(i as u32 + 1), format!("{label} in the tree — no secrets in the root, ever; reference credentials by name in the instance's vault"));
                }
            }
            if soft.is_match(line) {
                sink.warning(ctx, 10, rel, Some(i as u32 + 1), "high-entropy value assigned to a credential-shaped key — verify this is not a secret".into());
            }
        }
    };

    for a in &ctx.tree.artifacts {
        scan(&a.rel, &a.text, sink);
    }
    for rel in &ctx.tree.other_files {
        let abs = ctx.tree.root.path.join(rel);
        let Ok(bytes) = std::fs::read(&abs) else {
            continue;
        };
        if bytes.len() > 512 * 1024 || bytes[..bytes.len().min(8192)].contains(&0) {
            continue; // binary or oversized — not a text secret carrier we scan
        }
        let text = String::from_utf8_lossy(&bytes);
        scan(rel, &text, sink);
    }
}

// 11. No grouping-rather-than-kind directory.
fn item11(ctx: &Ctx, sink: &mut Sink) {
    if ctx.tree.has_dir("skills") {
        sink.violation(ctx, 11, "skills", None, "root-level skills/ is a grouping, not a kind — procedures live in solution/{bc}/skills/ or org/{role}/holder/skills/ (decision 0021)".into());
    }
    for named in ["functions", "departments", "personas", "requirements"] {
        if ctx.tree.has_dir(named) {
            sink.violation(ctx, 11, named, None, format!("root-level {named}/ is a grouping, not a kind (spec rule 7) — groupings are tags; views over tags are generated"));
        }
    }
    for dir in ctx.tree.subdirs("plans") {
        sink.violation(ctx, 11, dir, None, "plans/{plan}/ directories are a grouping violation — sub-plans are flat siblings (plans/{parent}-{piece}.md, decision 0026)".into());
    }
}

// 12. metrics/actuals/ content within the freshness window from rituals.md.
fn item12(ctx: &Ctx, sink: &mut Sink) {
    let Some(window) = ctx.reg.freshness_window_days else {
        if ctx.tree.get("rituals.md").is_none() {
            sink.judgment(
                12,
                "no rituals.md in this root — the actuals freshness window is undeclared",
                vec![],
            );
        }
        return;
    };
    let actuals: Vec<_> = ctx.tree.by_kind(Kind::MetricsActual).collect();
    if actuals.is_empty() {
        sink.warning(ctx, 12, "metrics/actuals", None, format!("metrics/actuals/ holds no content — the freshness window is {window} days and there is nothing within it"));
        return;
    }
    let newest = actuals
        .iter()
        .filter_map(|a| {
            a.fm.as_ref()
                .and_then(|fm| fm.get_str("date"))
                .and_then(|d| Date::parse(&d))
                .or_else(|| ctx.git.last_commit_date(&a.rel))
        })
        .max();
    let Some(newest) = newest else { return };
    let age = dates::days_between(newest, ctx.today);
    if age > window {
        let rel = actuals
            .iter()
            .map(|a| a.rel.clone())
            .next()
            .unwrap_or_else(|| "metrics/actuals".into());
        sink.violation(ctx, 12, &rel, None, format!("newest actuals are {age} days old — outside the {window}-day freshness window; agents never reason from numbers older than the interval (spec rule 5)"));
    }
}

// 13. Strategy frontmatter: legal status, resolving need anchor, resolving
// supersedes, committed band declares differentiation.
fn item13(ctx: &Ctx, sink: &mut Sink) {
    for a in ctx.tree.by_kind(Kind::Strategy) {
        let Some(fm) = &a.fm else { continue };
        let status = fm.get_str("status");
        match &status {
            None => sink.violation(ctx, 13, &a.rel, Some(1), "strategy declares no status: (raw | defined | validated | implemented | established | discarded)".into()),
            Some(s) if StrategyStatus::parse(s).is_none() => sink.violation(ctx, 13, &a.rel, fm.line_of("status"), format!("status: {s} is not on the maturity ladder (raw | defined | validated | implemented | established | discarded)")),
            _ => {}
        }
        match fm.get_str("need") {
            None => sink.violation(ctx, 13, &a.rel, Some(1), "strategy declares no need: — every strategy anchors to a market.md need (market.md#n-{slug})".into()),
            Some(need) => {
                if let Err(reason) = refs::resolve(&need, ctx.tree) {
                    sink.violation(ctx, 13, &a.rel, fm.line_of("need"), format!("need: {need} does not resolve — {reason}"));
                }
            }
        }
        if let Some(sup) = fm.get_str("supersedes") {
            if ctx.tree.get(&sup).is_none() {
                sink.violation(
                    ctx,
                    13,
                    &a.rel,
                    fm.line_of("supersedes"),
                    format!("supersedes: {sup} does not resolve"),
                );
            }
        }
        let committed = status
            .as_deref()
            .and_then(StrategyStatus::parse)
            .map(|s| s.band() == Band::Committed)
            .unwrap_or(false);
        if committed && fm.get_str("differentiation").is_none() {
            sink.violation(ctx, 13, &a.rel, Some(1), "committed-band strategy declares no differentiation: — why we win, against which alternative".into());
        }
    }
}

// 14. Every subdomain has an edge to a committed-band strategy.
fn item14(ctx: &Ctx, sink: &mut Sink) {
    for orphan in ctx.derived.orphan_subdomains() {
        // Only meaningful when the edges themselves are well-formed enough
        // to know they point outside the committed band; item 3 owns shape.
        sink.violation(ctx, 14, orphan, None, "no edge to a committed-band strategy — orphan subdomain; escalate for re-parenting or archival (carries core policy until resolved)".into());
    }
}

// 15. >1 core edge under a committed strategy requires core-ranking: — a
// total order covering exactly its core subdomains.
fn item15(ctx: &Ctx, sink: &mut Sink) {
    for strategy in ctx.derived.committed() {
        let mut core_subs: Vec<&str> = ctx
            .derived
            .induced
            .iter()
            .filter(|(_, edges)| {
                edges
                    .iter()
                    .any(|e| e.strategy == strategy && e.class == Some(Class::Core))
            })
            .map(|(s, _)| s.as_str())
            .collect();
        core_subs.sort();
        if core_subs.len() <= 1 {
            continue;
        }
        let Some(a) = ctx.tree.get(strategy) else {
            continue;
        };
        let ranking = a.fm.as_ref().and_then(|fm| fm.get_list("core-ranking"));
        match ranking {
            None => sink.violation(ctx, 15, strategy, Some(1), format!("{} core subdomains and no core-ranking: — declare a total order, scarcest attention first", core_subs.len())),
            Some(r) => {
                let mut sorted = r.clone();
                sorted.sort();
                sorted.dedup();
                if sorted != core_subs || r.len() != core_subs.len() {
                    sink.violation(
                        ctx, 15, strategy,
                        a.fm.as_ref().and_then(|fm| fm.line_of("core-ranking")),
                        format!("core-ranking: must cover exactly its core subdomains, once each — expected a total order over {{{}}}", core_subs.join(", ")),
                    );
                }
            }
        }
    }
}

fn item16(_ctx: &Ctx, sink: &mut Sink) {
    sink.judgment(16, "market.md technology-freedom (strategy vocabulary, chosen means, stack names) is a judgment check", vec![]);
}

// 17. Discarded strategy with no committed band = incomplete pivot.
fn item17(ctx: &Ctx, sink: &mut Sink) {
    if !ctx.derived.committed().is_empty() {
        return;
    }
    let mut discarded: Vec<&String> = ctx
        .derived
        .strategy_status
        .iter()
        .filter(|(_, s)| **s == Some(StrategyStatus::Discarded))
        .map(|(k, _)| k)
        .collect();
    discarded.sort();
    for s in discarded {
        sink.violation(ctx, 17, s, None, "discarded with no strategy in the committed band — incomplete pivot; commit a successor or archive the induced subdomains".into());
    }
}

// 18. Spec version pinned in decisions/0000-adopt-trellis.md matches the
// kernel's embedded spec version.
fn item18(ctx: &Ctx, sink: &mut Sink) {
    let rel = "decisions/0000-adopt-trellis.md";
    let Some(a) = ctx.tree.get(rel) else {
        sink.violation(
            ctx,
            18,
            "decisions",
            None,
            "no decisions/0000-adopt-trellis.md — the root pins no spec version".into(),
        );
        return;
    };
    let re = regex::Regex::new(r"(?i)spec(?:ification)?[,]?\s+v(\d+)").unwrap();
    match re.captures(&a.text) {
        None => sink.violation(
            ctx,
            18,
            rel,
            None,
            "adopt decision pins no spec version (expected `specification vN`)".into(),
        ),
        Some(cap) => {
            let pinned: u32 = cap[1].parse().unwrap_or(0);
            if pinned != ctx.spec_version {
                sink.violation(ctx, 18, rel, None, format!("root is pinned to spec v{pinned}, current is v{} — review the intervening changes and re-pin, or record a deviation decision", ctx.spec_version));
            }
        }
    }
}

// 19. funded-by edge legality.
fn item19(ctx: &Ctx, sink: &mut Sink) {
    for a in ctx.tree.by_kind(Kind::Strategy) {
        let Some(fm) = &a.fm else { continue };
        let line = fm.line_of("funded-by");
        for edge in ctx
            .derived
            .funded
            .get(&a.rel)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
        {
            if edge.target.is_empty() {
                sink.violation(
                    ctx,
                    19,
                    &a.rel,
                    line,
                    "funded-by: edge names no strategy".into(),
                );
                continue;
            }
            if edge.target == a.rel {
                sink.violation(ctx, 19, &a.rel, line, format!("funded-by: names its own path ({}) — a strategy that captures its own revenue declares `self`", a.rel));
            } else if let refs::RefKind::Path { path, .. } = refs::classify(&edge.target) {
                if !path.starts_with("strategy/") {
                    sink.violation(ctx, 19, &a.rel, line, format!("funded-by: target {path} is not `self`, a strategy/{{strategy}}.md ref, or a declared external ref"));
                } else if ctx.tree.get(&path).is_none() {
                    sink.violation(
                        ctx,
                        19,
                        &a.rel,
                        line,
                        format!("funded-by: target {path} does not resolve"),
                    );
                }
            }
            if let Some(rel_raw) = &edge.raw_relation {
                if rel_raw != "current" && rel_raw != "intended" {
                    sink.violation(ctx, 19, &a.rel, line, format!("relation: {rel_raw} is not legal (current | intended; absent means current)"));
                }
            }
        }
    }
}

// 20. Every committed-band strategy is economically sustained.
fn item20(ctx: &Ctx, sink: &mut Sink) {
    for orphan in ctx.derived.economic_orphans() {
        sink.violation(ctx, 20, orphan, None, "committed with no sustaining funded-by: edge (`current`, targeting self, a committed strategy, or an external ref) — economic orphan; re-fund it, commit the conversion it intends, or reconsider the commitment".into());
    }
}

// 21. The committed band has a capture point.
fn item21(ctx: &Ctx, sink: &mut Sink) {
    let committed = ctx.derived.committed();
    if committed.is_empty() || ctx.derived.capture_point_exists() {
        return;
    }
    // The economically-orphaned members already fire item 20; the closed
    // loop is one finding, anchored on the band's first member.
    let anchor = committed[0];
    sink.violation(ctx, 21, anchor, None, format!("the committed band ({}) has no capture point — no member captures its own revenue (`current` `self` edge) or is sustained from outside the root; value circulates, nothing enters", committed.join(", ")));
}

// 22. The awaits: graph over plans/ is acyclic.
fn item22(ctx: &Ctx, sink: &mut Sink) {
    for cycle in ctx.derived.awaits_cycles() {
        let named = cycle.join(" → ");
        for member in &cycle {
            sink.violation(ctx, 22, member, None, format!("awaits: cycle — {named} — a deadlock by construction: dispatch holds every member until the others retire, which none can; drop an edge or split a plan"));
        }
    }
}

// 23. Every context-map relation is backed by the language it claims —
// mechanical half: published-language needs a non-empty contracts/;
// conformist/acl need a ref in notes.
fn item23(ctx: &Ctx, sink: &mut Sink) {
    let Some(map) = ctx.tree.get("solution/context-map.md") else {
        return;
    };
    let skip = map.fm.as_ref().map(|f| f.close_line).unwrap_or(0);
    let mut checked = Vec::new();
    for table in crate::markdown::tables(&map.text, skip) {
        let header: Vec<String> = table.header.iter().map(|h| h.to_lowercase()).collect();
        let col = |n: &str| header.iter().position(|h| h == n);
        let (Some(ci_ctx), Some(ci_rel)) = (col("context"), col("relation")) else {
            continue;
        };
        let ci_kind = col("kind");
        let ci_notes = col("notes");
        for (cells, line) in &table.rows {
            let context = cells.get(ci_ctx).cloned().unwrap_or_default();
            let relation = cells
                .get(ci_rel)
                .cloned()
                .unwrap_or_default()
                .to_lowercase();
            let kind = ci_kind
                .and_then(|i| cells.get(i))
                .cloned()
                .unwrap_or_default();
            let notes = ci_notes
                .and_then(|i| cells.get(i))
                .cloned()
                .unwrap_or_default();
            if relation.contains("published-language") {
                checked.push(context.clone());
                let internal_has_contracts = |bc: &str| -> bool {
                    let dir = format!("solution/{bc}/contracts");
                    ctx.tree
                        .artifacts
                        .iter()
                        .map(|a| &a.rel)
                        .chain(ctx.tree.other_files.iter())
                        .any(|f| f.starts_with(&format!("{dir}/")))
                };
                let backed = if kind == "internal" {
                    internal_has_contracts(&context)
                } else {
                    // The publisher is on this root's side; some internal
                    // context must hold the published language.
                    ctx.tree.subdirs("solution").iter().any(|d| {
                        d.strip_prefix("solution/")
                            .map(internal_has_contracts)
                            .unwrap_or(false)
                    })
                };
                if !backed {
                    sink.violation(ctx, 23, "solution/context-map.md", Some(*line), format!("relation with {context} claims published-language but no publishing context's contracts/ holds an artifact — a promise with no artifact, consumers conforming to conversation"));
                }
            } else if relation.contains("conformist") || relation.contains("acl") {
                checked.push(context.clone());
                if notes.trim().is_empty() || notes.trim() == "—" {
                    sink.violation(ctx, 23, "solution/context-map.md", Some(*line), format!("relation with {context} claims {relation} but carries no ref to the upstream language — conforming to something unnamed"));
                }
            }
        }
    }
    sink.judgment(23, "whether a contract artifact actually backs the language claimed, and whether refs are pinned per the boundary guarantees, is a judgment check", checked);
}

// 24. Escalation records well-formed; every blocked plan carries an open
// record; no record in a generated artifact.
fn item24(ctx: &Ctx, sink: &mut Sink) {
    for a in &ctx.tree.artifacts {
        let records = escalation_records(a);
        // Non-yaml fences under an Escalations heading are malformed records.
        for fence in &a.fences {
            if let Some(section) = fence.section {
                if a.headings
                    .get(section)
                    .map(|h| h.slug == "escalations")
                    .unwrap_or(false)
                    && !fence.lang.eq_ignore_ascii_case("yaml")
                {
                    sink.violation(ctx, 24, &a.rel, Some(fence.open_line), "block under ## Escalations is not fenced yaml — records follow the schema in conventions.md".into());
                }
            }
        }
        if !records.is_empty() && a.provenance().as_deref() == Some("generated") {
            sink.violation(ctx, 24, &a.rel, None, "escalation records in a generated artifact — escalations against generated output belong to its source".into());
        }
        for r in &records {
            match &r.status {
                None => sink.violation(
                    ctx,
                    24,
                    &a.rel,
                    Some(r.line),
                    "escalation record declares no status: (open | resolved)".into(),
                ),
                Some(s) if s != "open" && s != "resolved" => sink.violation(
                    ctx,
                    24,
                    &a.rel,
                    Some(r.line),
                    format!("escalation status: {s} is not legal (open | resolved)"),
                ),
                _ => {}
            }
            match &r.raised {
                None => sink.violation(
                    ctx,
                    24,
                    &a.rel,
                    Some(r.line),
                    "escalation record declares no raised: date".into(),
                ),
                Some(d) if Date::parse(d).is_none() => sink.violation(
                    ctx,
                    24,
                    &a.rel,
                    Some(r.line),
                    format!("raised: {d} is not a YYYY-MM-DD date"),
                ),
                _ => {}
            }
            for (field, value) in [("by", &r.by), ("to", &r.to)] {
                match value {
                    None => sink.violation(
                        ctx,
                        24,
                        &a.rel,
                        Some(r.line),
                        format!("escalation record declares no {field}: role"),
                    ),
                    Some(role) => {
                        if !ctx.tree.role_exists(role) {
                            sink.violation(
                                ctx,
                                24,
                                &a.rel,
                                Some(r.line),
                                format!("{field}: {role} does not resolve to a role under org/"),
                            );
                        }
                    }
                }
            }
        }
        if a.kind == Kind::Plan && a.status().as_deref() == Some("blocked") {
            let open = records.iter().any(|r| r.status.as_deref() == Some("open"));
            if !open {
                sink.violation(ctx, 24, &a.rel, None, "blocked plan carries no open escalation record — blocked asserts a defect a human must clear; state the blocker or flip the status back (a held plan stays ready and owes no record)".into());
            }
        }
    }
}

// 25. The terminal tier is well-formed.
fn item25(ctx: &Ctx, sink: &mut Sink) {
    for a in &ctx.tree.artifacts {
        // The tier is never an address: refs name the live path, whichever
        // side of the move the target is on (spec rule 13). Frontmatter has
        // no legitimate use for the prefix, so a plain scan of the block is
        // both complete and free of false positives.
        if let Some(fm) = &a.fm {
            for (i, line) in a.text.lines().take(fm.close_line as usize).enumerate() {
                if line.contains(crate::tree::ARCHIVE) {
                    sink.violation(ctx, 25, &a.rel, Some(i as u32 + 1), format!("ref names the terminal tier ({}…) — refs name the live path, which keeps resolving after a move; drop the prefix", crate::tree::ARCHIVE));
                    break;
                }
            }
        }

        if !a.archived {
            continue;
        }

        // The mirror is the whole convention: an archived path is a live
        // path with the prefix in front. Anything that does not classify
        // through it is filed where nothing will ever look for it.
        if a.kind == Kind::Other {
            sink.violation(ctx, 25, &a.rel, None, format!("{} holds a path that is not a mirror of the live hierarchy — an archived artifact's path is its live path with the prefix in front, so nothing else belongs here", crate::tree::ARCHIVE));
            continue;
        }

        // Decisions are shared memory across agent generations (spec rule 6);
        // append-only means they are never filed away either.
        if a.kind == Kind::Decision {
            sink.violation(ctx, 25, &a.rel, None, "decisions are never archived — decisions/ is append-only shared memory (spec rule 6); supersede with a new numbered decision instead".into());
            continue;
        }

        // Admission is terminal-only. A unit that archives as a subtree
        // (a bounded context, a role) carries the marker on its entry
        // artifact; the rest of the subtree rides along.
        let Some(want) = crate::tree::terminal_status(a.kind, &a.rel) else {
            continue;
        };
        let got = a.status();
        if got.as_deref() != Some(want) {
            sink.violation(ctx, 25, &a.rel, a.fm.as_ref().and_then(|f| f.line_of("status")), format!("archived artifact declares status: {} — the tier admits terminal artifacts only, which for this kind means status: {want}", got.as_deref().unwrap_or("(none)")));
        }
    }
}
