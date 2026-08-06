//! Generated views: pure functions of the tree, git history, and today's
//! date. Deterministic by construction — lexicographic iteration, no
//! wall-clock except the `date:` field — so same tree + same HEAD + same day
//! renders byte-identical output, which is what makes lint item 2 mechanical
//! for these files (`generated-by:` stamp + regenerate + diff).

use std::collections::BTreeMap;

use crate::dates::{self, Date};
use crate::gitio::Git;
use crate::graph;
use crate::model::{escalation_records, PlanStatus};
use crate::registries;
use crate::tree::{Kind, Tree};

pub const NAMES: &[&str] = &["board", "codeowners", "tags", "orgchart", "escalations"];

pub fn render_named(name: &str, tree: &Tree, git: &Git, today: Date) -> Option<String> {
    match name {
        "board" => Some(board(tree, git, today)),
        "codeowners" => Some(codeowners(tree)),
        "tags" => Some(tags(tree)),
        "orgchart" => Some(orgchart(tree)),
        "escalations" => Some(escalations(tree)),
        _ => None,
    }
}

/// Canonical `--write` destination, where the spec places one.
pub fn default_path(name: &str) -> Option<&'static str> {
    match name {
        "board" => Some("metrics/actuals/plan-board.md"),
        "codeowners" => Some(".github/CODEOWNERS"),
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
        match status.as_str() {
            "ready" if days > dispatch_cadence && derived.hold(&p.rel).is_none() => {
                flags.push("stalled queue".into());
            }
            "blocked" if days > window => flags.push("stalled blocker".into()),
            "active" => {
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
        out.push_str("\n(no tags registered in conventions.md)\n");
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
        let holder = if let Some(r) = tree
            .get(&format!("org/{role}/holder/ref.md"))
            .and_then(|a| a.fm.as_ref())
            .and_then(|f| f.get_str("ref"))
        {
            format!("ref: {r}")
        } else if tree.get(&format!("org/{role}/holder/system.md")).is_some() {
            "agent package".to_string()
        } else {
            fm.and_then(|f| f.get_str("holder"))
                .filter(|h| h != "holder/")
                .map(|h| format!("external: {h}"))
                .unwrap_or_else(|| "(none)".into())
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
