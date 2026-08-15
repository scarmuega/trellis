//! Generated views: pure functions of the tree, git history, and today's
//! date. Deterministic by construction — lexicographic iteration, no
//! wall-clock except the `date:` field — so same tree + same HEAD + same day
//! renders byte-identical output, which is what makes lint item 2 mechanical
//! for these files (`generated-by:` stamp + regenerate + diff).

use std::collections::{BTreeMap, BTreeSet};

use crate::dates::{self, Date};
use crate::gitio::Git;
use crate::graph;
use crate::model::{escalation_records, PlanStatus};
use crate::registries;
use crate::tree::{Kind, Tree};

pub const NAMES: &[&str] = &[
    "board",
    "plan-graph",
    "codeowners",
    "tags",
    "orgchart",
    "escalations",
    "decisions",
];

pub fn render_named(name: &str, tree: &Tree, git: &Git, today: Date) -> Option<String> {
    match name {
        "board" => Some(board(tree, git, today)),
        "plan-graph" => Some(plan_graph(tree, today)),
        "codeowners" => Some(codeowners(tree)),
        "tags" => Some(tags(tree)),
        "orgchart" => Some(orgchart(tree)),
        "escalations" => Some(escalations(tree)),
        "decisions" => Some(decisions(tree)),
        _ => None,
    }
}

/// Canonical `--write` destination, where the spec places one.
pub fn default_path(name: &str) -> Option<&'static str> {
    match name {
        "board" => Some("metrics/actuals/plan-board.md"),
        "plan-graph" => Some("metrics/actuals/plan-graph.md"),
        "codeowners" => Some(".github/CODEOWNERS"),
        "decisions" => Some("metrics/actuals/decision-register.md"),
        _ => None,
    }
}

fn fm_header(name: &str, today: Date) -> String {
    format!(
        "---\nprovenance: generated\nowner: org/steward\ndate: {today}\ngenerated-by: trellis view {name}\n---\n"
    )
}

const STATUSES: [PlanStatus; 5] = [
    PlanStatus::Draft,
    PlanStatus::Ready,
    PlanStatus::Active,
    PlanStatus::Blocked,
    PlanStatus::Retired,
];

/// Plan board: census / flow / dwell / mix from `plans/` frontmatter and git
/// history alone (Execution health pattern, decision 0030 — no
/// percent-complete, ever).
pub fn board(tree: &Tree, git: &Git, today: Date) -> String {
    let derived = graph::derive(tree);
    let reg = registries::load(tree);
    let window = reg.freshness_window_days.unwrap_or(7);
    let dispatch_cadence = reg
        .rituals
        .iter()
        .find(|r| r.name.eq_ignore_ascii_case("plan dispatch"))
        .and_then(|r| registries::cadence_days(&r.cadence))
        .unwrap_or(1);

    let mut plans: Vec<_> = tree.by_kind(Kind::Plan).collect();
    plans.sort_by(|a, b| a.rel.cmp(&b.rel));

    let mut out = fm_header("board", today);
    out.push_str("# Plan board\n\n");
    out.push_str("Census, flow, dwell, and mix over `plans/`, computed from frontmatter and\ngit history alone (Execution health pattern). Dwell and movement are the\nsignals; there is no percent-complete.\n\n");

    // Census.
    out.push_str("## Census\n\n| status | count |\n|--------|-------|\n");
    for s in STATUSES {
        let n = plans
            .iter()
            .filter(|p| p.status().as_deref() == Some(s.as_str()))
            .count();
        out.push_str(&format!("| {} | {} |\n", s.as_str(), n));
    }

    let cut = |label: &str, f: &dyn Fn(&&crate::tree::Artifact) -> String, out: &mut String| {
        let mut groups: BTreeMap<String, [usize; 5]> = BTreeMap::new();
        for p in &plans {
            let key = f(p);
            let counts = groups.entry(key).or_default();
            if let Some(idx) = STATUSES
                .iter()
                .position(|s| p.status().as_deref() == Some(s.as_str()))
            {
                counts[idx] += 1;
            }
        }
        if groups.is_empty() {
            return;
        }
        out.push_str(&format!(
            "\nBy {label}:\n\n| {label} | draft | ready | active | blocked | retired |\n|---|---|---|---|---|---|\n"
        ));
        for (key, counts) in groups {
            out.push_str(&format!(
                "| {key} | {} | {} | {} | {} | {} |\n",
                counts[0], counts[1], counts[2], counts[3], counts[4]
            ));
        }
    };
    cut(
        "type",
        &|p| {
            p.fm.as_ref()
                .and_then(|f| f.get_str("type"))
                .unwrap_or_else(|| "—".into())
        },
        &mut out,
    );
    cut(
        "owner",
        &|p| p.owner().unwrap_or_else(|| "—".into()),
        &mut out,
    );
    cut(
        "class",
        &|p| {
            derived
                .plan_class(tree, &p.rel)
                .map(|c| c.as_str().to_string())
                .unwrap_or_else(|| "—".into())
        },
        &mut out,
    );

    // Flow.
    let in_window =
        |d: Date| dates::days_between(d, today) <= window && dates::days_between(d, today) >= 0;
    let mut authored = 0usize;
    let mut released = 0usize;
    let mut closed = 0usize;
    for p in &plans {
        if git
            .first_commit_date(&p.rel)
            .map(&in_window)
            .unwrap_or(false)
        {
            authored += 1;
        }
        let timeline = git.status_timeline(&p.rel);
        for pair in timeline.windows(2) {
            let (prev, (date, next)) = (&pair[0].1, &pair[1]);
            if !in_window(*date) {
                continue;
            }
            let was_draft = prev.as_deref() == Some("draft");
            match next.as_deref() {
                Some("ready") if was_draft => released += 1,
                Some("active") if was_draft => released += 1,
                Some("retired") => closed += 1,
                _ => {}
            }
        }
    }
    out.push_str(&format!(
        "\n## Flow — last {window} days\n\n| event | count |\n|-------|-------|\n| authored | {authored} |\n| released | {released} |\n| closed | {closed} |\n"
    ));

    // Dwell.
    out.push_str(
        "\n## Dwell\n\n| plan | status | days | flag |\n|------|--------|------|------|\n",
    );
    let mut rows: Vec<(String, String, i64, String)> = Vec::new();
    let horizon_re = regex::Regex::new(r"(?im)^horizon:?\s*(\d+)\s*days?").unwrap();
    for p in &plans {
        let Some(status) = p.status() else { continue };
        if !matches!(status.as_str(), "ready" | "active" | "blocked") {
            continue;
        }
        let days = git
            .status_set_date(&p.rel, &status)
            .map(|d| dates::days_between(d, today))
            .unwrap_or(0);
        let mut flags: Vec<String> = Vec::new();
        if let Some((target, _)) = derived.hold(&p.rel) {
            flags.push(format!("held — awaits {target}"));
        }
        // The dispatch precheck's hold, on the same card (decision 0045): a
        // ready plan failing mechanical readiness never dispatches, and a
        // queue it cannot leave should say why rather than read as stalled.
        let readiness_held = status == "ready"
            && crate::readiness::check(tree, &derived, &p.rel)
                .map(|r| {
                    if r.mechanical_pass {
                        None
                    } else {
                        Some(
                            r.items
                                .iter()
                                .filter(|i| i.status == crate::readiness::ItemStatus::Fail)
                                .map(|i| i.item.to_string())
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                    }
                })
                .ok()
                .flatten()
                .inspect(|items| flags.push(format!("held — readiness item(s) {items}")))
                .is_some();
        match status.as_str() {
            "ready" if days > dispatch_cadence && derived.hold(&p.rel).is_none() && !readiness_held => {
                flags.push("stalled queue".into());
            }
            "blocked" if days > window => flags.push("stalled blocker".into()),
            "active" => {
                // Parked on a human, not stalled by an agent: the dwell is
                // real, and so is the reason for it.
                if let Some(handoff) = p.fm.as_ref().and_then(|f| f.get_str("handoff")) {
                    flags.push(format!("parked — handoff {handoff}"));
                }
                let horizon = horizon_re
                    .captures(&p.text)
                    .and_then(|c| c[1].parse::<i64>().ok());
                if let Some(h) = horizon {
                    if days > h {
                        flags.push(format!("owed verdict — horizon {h}d"));
                    }
                }
            }
            _ => {}
        }
        rows.push((p.rel.clone(), status, days, flags.join("; ")));
    }
    rows.sort_by(|a, b| b.2.cmp(&a.2).then(a.0.cmp(&b.0)));
    if rows.is_empty() {
        out.push_str("| (no ready, active, or blocked plans) | | | |\n");
    }
    for (rel, status, days, flag) in rows {
        out.push_str(&format!("| {rel} | {status} | {days} | {flag} |\n"));
    }

    // Mix.
    let wip: Vec<_> = plans
        .iter()
        .filter(|p| matches!(p.status().as_deref(), Some("active") | Some("blocked")))
        .collect();
    let mut per_owner: BTreeMap<String, usize> = BTreeMap::new();
    for p in &wip {
        *per_owner
            .entry(p.owner().unwrap_or_else(|| "—".into()))
            .or_default() += 1;
    }
    out.push_str(&format!(
        "\n## Mix\n\n- WIP (active + blocked): {}\n",
        wip.len()
    ));
    if !per_owner.is_empty() {
        let s: Vec<String> = per_owner.iter().map(|(o, n)| format!("{o}: {n}")).collect();
        out.push_str(&format!("- WIP by owner: {}\n", s.join(", ")));
    }
    out.push_str(&format!("- closures in window: {closed}\n"));
    if closed > 0 {
        let cycle = wip.len() as f64 * window as f64 / closed as f64;
        out.push_str(&format!(
            "- cycle time (WIP / closure rate): {cycle:.0} days\n"
        ));
    } else {
        out.push_str("- cycle time: — (no closures in window)\n");
    }
    out
}

fn github_handle(tree: &Tree, role_ref: &str) -> Option<String> {
    let role = role_ref.strip_prefix("org/")?;
    tree.get(&format!("org/{role}/holder/ref.md"))
        .and_then(|a| a.fm.as_ref())
        .and_then(|fm| fm.get_str("github"))
        .map(|h| {
            if h.starts_with('@') {
                h
            } else {
                format!("@{h}")
            }
        })
}

/// The class a node carries, in the order the legend and the `classDef` lines
/// emit them. There is no `retired` entry: a retired plan is off the board, so
/// the graph never draws one. `unknown` is a plan whose `status:` is absent or
/// unparseable; `missing` is an `awaits:` target that resolves to no plan at
/// all (lint item 4 owns that complaint — the picture shows the hole rather
/// than dropping the edge). `cycle` is additive: a member of an `awaits:` cycle
/// keeps its status class and gains this one, so item 22's deadlock is visible,
/// not just reported.
const NODE_CLASSES: [(&str, &str); 7] = [
    ("draft", "stroke:#9e9e9e,stroke-dasharray:3 3"),
    ("ready", "stroke:#1e88e5,stroke-width:2px"),
    ("active", "stroke:#43a047,stroke-width:3px"),
    ("blocked", "stroke:#fb8c00,stroke-width:3px"),
    ("unknown", "stroke:#9e9e9e,stroke-width:1px"),
    (
        "missing",
        "stroke:#e53935,stroke-width:2px,stroke-dasharray:4 3",
    ),
    ("cycle", "stroke:#e53935,stroke-width:4px"),
];

/// `3 edges` / `1 edge` — the counts under the figure read as prose, and a
/// root with exactly one of something is the common case, not the exotic one.
fn count(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("1 {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

/// `plans/2026/0012-seasonal-line.md` → `2026/0012-seasonal-line`. The `plans/`
/// prefix is on every node, so it carries no information inside the graph.
fn node_label(rel: &str) -> String {
    rel.strip_prefix("plans/")
        .unwrap_or(rel)
        .strip_suffix(".md")
        .unwrap_or(rel)
        .replace('"', "#quot;")
}

/// Plan graph: the `awaits:` DAG as a Mermaid flowchart, so the shape is
/// legible at all. The text surfaces this graph one edge at a time — `plan
/// list --held`, `readiness`, the board's dwell flag — which never shows a
/// chain's length or that one unretired plan is holding five others.
///
/// Arrows point dependent → target: the edge carries the declaration, so an
/// arrow reads "waits on", and following one leads to what must finish first.
///
/// Every live plan is a node, sequenced or not — the graph is the standing
/// picture of the board, and a plan free of ordering constraints is a fact
/// worth seeing, not an omission. Retired plans are the one exclusion: they
/// hold nothing and wait on nothing, so drawing them would bury the live
/// picture under the whole history. An edge onto a retired target drops with
/// it, which is the same statement its hold having cleared.
///
/// Styling is stroke-only — no `fill`, no `color` — so the figure reads under
/// both the light and the dark renderer theme, and status is encoded in stroke
/// width and dash pattern as well as hue.
pub fn plan_graph(tree: &Tree, today: Date) -> String {
    let derived = graph::derive(tree);

    let retired = |p: &str| derived.plan_status.get(p) == Some(&Some(PlanStatus::Retired));
    let mut plans: Vec<&str> = derived.plan_awaits.keys().map(|s| s.as_str()).collect();
    plans.sort();

    // Every live plan is a node before any edge is read.
    let mut nodes: BTreeSet<String> = plans
        .iter()
        .filter(|p| !retired(p))
        .map(|p| p.to_string())
        .collect();

    // `(dependent, target)` — the arrow's direction, the declaration's own.
    // Keys are already live paths (graph.rs), so an edge written before its
    // target was archived still lands on one node. A target that resolves to
    // nothing is drawn anyway (lint item 4 owns the complaint); one that has
    // retired is not, and its edge goes with it.
    let mut edges: Vec<(String, String)> = Vec::new();
    for plan in plans.iter().filter(|p| !retired(p)) {
        for target in &derived.plan_awaits[*plan] {
            let target = crate::tree::live_path(target.trim()).to_string();
            if retired(&target) {
                continue;
            }
            nodes.insert(target.clone());
            edges.push((plan.to_string(), target));
        }
    }
    edges.sort();
    edges.dedup();

    let mut out = fm_header("plan-graph", today);
    out.push_str("# Plan graph\n\n");
    out.push_str("The `awaits:` DAG over `plans/`. An arrow runs from a plan to what it is\nwaiting for; a plan with no arrows is under no ordering constraint. Retired\nplans are left out — they hold nothing and wait on nothing. Red is a cycle\n(lint item 22) or an `awaits:` target that resolves to nothing (item 4).\n\n");

    let unsequenced = nodes
        .iter()
        .filter(|n| !edges.iter().any(|(d, t)| d == *n || t == *n))
        .count();

    if nodes.is_empty() {
        out.push_str("(no live plans)\n");
    } else {
        let ids: BTreeMap<&str, String> = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), format!("p{i}")))
            .collect();
        let status_class = |node: &str| -> &'static str {
            match derived.plan_status.get(node) {
                None => "missing",
                Some(None) => "unknown",
                Some(Some(s)) => s.as_str(),
            }
        };
        let mut members: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for node in &nodes {
            members
                .entry(status_class(node))
                .or_default()
                .push(ids[node.as_str()].as_str());
        }
        for cycle in derived.awaits_cycles() {
            for member in cycle {
                if let Some(id) = ids.get(member.as_str()) {
                    members.entry("cycle").or_default().push(id.as_str());
                }
            }
        }

        out.push_str("```mermaid\nflowchart LR\n");
        for node in &nodes {
            out.push_str(&format!(
                "  {}[\"{}<br/>{}\"]\n",
                ids[node.as_str()],
                node_label(node),
                status_class(node)
            ));
        }
        for (dependent, target) in &edges {
            out.push_str(&format!(
                "  {} --> {}\n",
                ids[dependent.as_str()],
                ids[target.as_str()]
            ));
        }
        for (class, style) in NODE_CLASSES {
            let Some(ids) = members.get(class) else {
                continue;
            };
            let mut ids = ids.clone();
            ids.sort();
            ids.dedup();
            out.push_str(&format!("  classDef {class} {style}\n"));
            out.push_str(&format!("  class {} {class}\n", ids.join(",")));
        }
        out.push_str("```\n");
    }

    let held = plans.iter().filter(|p| derived.hold(p).is_some()).count();
    let cycles = derived.awaits_cycles().len();
    let unresolved = nodes
        .iter()
        .filter(|n| !derived.plan_status.contains_key(n.as_str()))
        .count();
    out.push_str(&format!(
        "\n- drawn: {}, {}\n- held: {}, waiting on a target that has not retired\n- unsequenced: {}, under no ordering constraint\n- retired: {}, left out of the figure\n",
        count(nodes.len(), "node"),
        count(edges.len(), "edge"),
        count(held, "ready plan"),
        count(unsequenced, "plan"),
        count(plans.iter().filter(|p| retired(p)).count(), "plan"),
    ));
    if unresolved > 0 {
        out.push_str(&format!(
            "- unresolved: {}, naming no plan in this root (lint item 4)\n",
            count(unresolved, "`awaits:` target")
        ));
    }
    if cycles > 0 {
        out.push_str(&format!(
            "- cycles: {} — a deadlock by construction (lint item 22)\n",
            count(cycles, "cycle")
        ));
    }
    out
}

/// CODEOWNERS over `owner:` frontmatter — entries only for core-class paths
/// (the binding's promise: a human review gate on every core change), plus
/// decisions/. Owners without a `github:` handle in their holder ref surface
/// as comment lines naming the gap.
pub fn codeowners(tree: &Tree) -> String {
    let derived = graph::derive(tree);
    let mut out = String::from(
        "# provenance: generated — trellis view codeowners\n# Core-class review gates derived from owner: frontmatter (automation\n# policy: an agent never lands a core change; branch protection plus this\n# file make the owner's review structural).\n\n",
    );
    let mut entries: Vec<(String, String)> = Vec::new(); // (path, owner ref)
    let mut gaps: Vec<(String, String)> = Vec::new();

    let push = |path: String,
                owner: Option<String>,
                entries: &mut Vec<(String, String)>,
                gaps: &mut Vec<(String, String)>| {
        let Some(owner) = owner else { return };
        match github_handle(tree, &owner) {
            Some(h) => entries.push((path, h)),
            None => gaps.push((path, owner)),
        }
    };

    // Archived artifacts are excluded throughout: a review gate on something
    // finished asks a human to approve changes to a file nobody should be
    // changing.
    for a in tree.by_kind(Kind::Problem) {
        if a.archived {
            continue;
        }
        if derived.effective_class(&a.rel) == crate::model::Class::Core {
            push(format!("/{}", a.rel), a.owner(), &mut entries, &mut gaps);
        }
    }
    for a in tree.by_kind(Kind::Plan) {
        if a.archived {
            continue;
        }
        if derived.plan_class(tree, &a.rel) == Some(crate::model::Class::Core) {
            push(format!("/{}", a.rel), a.owner(), &mut entries, &mut gaps);
            for bc in
                a.fm.as_ref()
                    .and_then(|f| f.get_list("contexts"))
                    .unwrap_or_default()
            {
                let bc = bc.trim_end_matches('/');
                let bc_owner = tree
                    .get(&format!("{bc}/README.md"))
                    .and_then(|r| r.owner())
                    .or_else(|| a.owner());
                push(format!("/{bc}/"), bc_owner, &mut entries, &mut gaps);
            }
        }
    }
    for a in tree.by_kind(Kind::Decision) {
        push(format!("/{}", a.rel), a.owner(), &mut entries, &mut gaps);
    }

    entries.sort();
    entries.dedup();
    gaps.sort();
    gaps.dedup();
    for (path, handle) in entries {
        out.push_str(&format!("{path} {handle}\n"));
    }
    for (path, owner) in gaps {
        out.push_str(&format!(
            "# {path} — {owner} declares no github: handle in org/{}/holder/ref.md\n",
            owner.strip_prefix("org/").unwrap_or(&owner)
        ));
    }
    out
}

/// Tag index over registered tags (unregistered tags are lint item 8's
/// business, not this view's).
pub fn tags(tree: &Tree) -> String {
    let reg = registries::load(tree);
    let today = crate::dates::today();
    let mut out = fm_header("tags", today);
    out.push_str("# Tag index\n");
    if reg.tags.is_empty() {
        out.push_str("\n(no tags registered in trellis.toml)\n");
        return out;
    }
    let mut sorted = reg.tags.clone();
    sorted.sort();
    for tag in sorted {
        out.push_str(&format!("\n## {tag}\n\n"));
        let mut carriers: Vec<&str> = tree
            .artifacts
            .iter()
            .filter(|a| {
                a.fm.as_ref()
                    .and_then(|fm| fm.get_list("tags"))
                    .map(|tags| tags.contains(&tag))
                    .unwrap_or(false)
            })
            .map(|a| a.rel.as_str())
            .collect();
        carriers.sort();
        if carriers.is_empty() {
            out.push_str("- (none)\n");
        }
        for c in carriers {
            out.push_str(&format!("- {c}\n"));
        }
    }
    out
}

/// Org chart from the org/ tree: purpose, holder shape, escalation edge.
pub fn orgchart(tree: &Tree) -> String {
    let today = crate::dates::today();
    let mut out = fm_header("orgchart", today);
    out.push_str("# Org chart\n\n");
    let mut roles = tree.roles();
    roles.sort();
    if roles.is_empty() {
        out.push_str("(no roles under org/)\n");
    }
    for role in roles {
        let mandate = tree.get(&format!("org/{role}/mandate.md"));
        let fm = mandate.and_then(|m| m.fm.as_ref());
        let purpose = fm
            .and_then(|f| f.get_str("purpose"))
            .unwrap_or_else(|| "—".into());
        let resolved = crate::org::holder(tree, &role);
        let holder = match (&resolved.kind, &resolved.reference) {
            (crate::org::HolderKind::Ref, Some(r)) => format!("ref: {r}"),
            (crate::org::HolderKind::Ref, None) => "ref: (unset)".to_string(),
            (crate::org::HolderKind::Package, _) => "agent package".to_string(),
            (crate::org::HolderKind::None, _) => fm
                .and_then(|f| f.get_str("holder"))
                .filter(|h| h != "holder/")
                .map(|h| format!("external: {h}"))
                .unwrap_or_else(|| "(none)".into()),
        };
        let escalate = fm
            .and_then(|f| f.get_str("escalate-to"))
            .unwrap_or_else(|| "—".into());
        out.push_str(&format!(
            "- **org/{role}** — {purpose}\n  - holder: {holder}\n  - escalates to: {escalate}\n"
        ));
    }
    out
}

/// Decision register: the live set over `decisions/`, read from the
/// successors' `supersedes:` edges (spec rule 6 — a decision is live iff no
/// accepted decision supersedes it), plus how each live decision is reached
/// from the operative record (Decision hatching pattern, lint item 26).
pub fn decisions(tree: &Tree) -> String {
    let derived = graph::derive(tree);
    let reg = registries::load(tree);
    let reach = graph::decision_reach(tree, &derived, &reg.decision_registry);
    let today = crate::dates::today();

    let mut out = fm_header("decisions", today);
    out.push_str("# Decision register\n\n");
    out.push_str("The live decision set — a reading, never a filing: a superseded decision\nstays in `decisions/` as record while its successor carries the guidance.\nA live decision unreachable from the operative record is the derivation\nsweep's orphaned-operative-content queue (lint item 26).\n\n");

    let live: Vec<_> = reach.iter().filter(|r| r.superseded_by.is_none()).collect();
    let superseded: Vec<_> = reach.iter().filter(|r| r.superseded_by.is_some()).collect();

    out.push_str("## Live\n\n");
    if live.is_empty() {
        out.push_str("(no accepted decisions)\n");
    } else {
        out.push_str("| decision | reachable via |\n|----------|---------------|\n");
        for r in &live {
            let via = if !r.cited_by.is_empty() {
                match r.cited_by.as_slice() {
                    [only] => format!("cited by {only}"),
                    [first, rest @ ..] => format!("cited by {first} (+{} more)", rest.len()),
                    [] => unreachable!(),
                }
            } else if r.inert {
                "inert (standing guidance: none)".into()
            } else if r.registered {
                "decision registry".into()
            } else {
                "unreachable".into()
            };
            out.push_str(&format!("| {} | {via} |\n", r.rel));
        }
    }

    out.push_str("\n## Superseded\n\n");
    if superseded.is_empty() {
        out.push_str("(none)\n");
    } else {
        out.push_str("| decision | superseded by |\n|----------|---------------|\n");
        for r in &superseded {
            out.push_str(&format!(
                "| {} | {} |\n",
                r.rel,
                r.superseded_by.as_deref().unwrap_or("—")
            ));
        }
    }

    let unreachable = live.iter().filter(|r| r.unreachable()).count();
    let drafts = tree
        .by_kind(Kind::Decision)
        .filter(|a| a.status().as_deref() != Some("accepted"))
        .count();
    out.push_str(&format!(
        "\n- live: {} of {} accepted\n- unreachable: {unreachable}\n",
        live.len(),
        reach.len()
    ));
    if drafts > 0 {
        out.push_str(&format!("- not yet accepted: {drafts}\n"));
    }
    out
}

/// Every open escalation record in the root — the generated view the runtime
/// binding names as the pull channel over records.
pub fn escalations(tree: &Tree) -> String {
    let today = crate::dates::today();
    let mut out = fm_header("escalations", today);
    out.push_str("# Open escalations\n\n");
    let mut rows: Vec<(String, String, String, String, String)> = Vec::new();
    for a in &tree.artifacts {
        for r in escalation_records(a) {
            if r.status.as_deref() != Some("open") {
                continue;
            }
            let f = |v: &Option<String>| v.clone().unwrap_or_else(|| "—".into());
            rows.push((a.rel.clone(), f(&r.raised), f(&r.by), f(&r.to), f(&r.asks)));
        }
    }
    rows.sort();
    if rows.is_empty() {
        out.push_str("(none open)\n");
        return out;
    }
    out.push_str(
        "| artifact | raised | by | to | asks |\n|----------|--------|----|----|------|\n",
    );
    for (artifact, raised, by, to, asks) in rows {
        out.push_str(&format!(
            "| {artifact} | {raised} | {by} | {to} | {asks} |\n"
        ));
    }
    out
}
