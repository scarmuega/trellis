//! Computed facts in the shapes the commands already print. Extracted from
//! `main.rs` so a second consumer — the daemon's HTTP surface — serves the
//! same JSON a command does. One shape per fact; a parallel encoding is the
//! drift decision 0037 exists to prevent.

use std::collections::HashSet;

use serde::Serialize;

use crate::dates::{self, Date};
use crate::gitio::Git;
use crate::graph::Derived;
use crate::model::escalation_records;
use crate::tree::{Kind, Scope, Tree};

/// The one wire encoding of a `Kind`. Every command that names a kind spells
/// it this way; a second spelling is the drift 0037 exists to prevent.
pub fn kind_name(kind: Kind) -> String {
    format!("{kind:?}").to_lowercase()
}

/// Open escalation records carried by one artifact.
pub fn open_escalations(tree: &Tree, rel: &str) -> usize {
    tree.get(rel)
        .map(|a| {
            escalation_records(a)
                .iter()
                .filter(|r| r.status.as_deref() == Some("open"))
                .count()
        })
        .unwrap_or(0)
}

/// `trellis show`: every computed fact about one artifact. `None` when the
/// path is not an artifact in this root.
pub fn artifact(
    tree: &Tree,
    git: &Git,
    derived: &Derived,
    rel: &str,
    today: Date,
) -> Option<serde_json::Value> {
    let a = tree.get(rel)?;
    let mut obj = serde_json::Map::new();
    let mut put = |k: &str, v: serde_json::Value| {
        obj.insert(k.to_string(), v);
    };
    put("path", rel.to_string().into());
    put("kind", kind_name(a.kind).into());
    put("provenance", a.provenance().into());
    put("owner", a.owner().into());
    if let Some(status) = a.status() {
        put("status", status.clone().into());
        if let Some(set) = git.status_set_date(rel, &status) {
            put("dwell_days", dates::days_between(set, today).into());
            put("status_since", set.to_string().into());
        }
    }
    match a.kind {
        Kind::Problem => {
            put(
                "effective_class",
                derived.effective_class(rel).as_str().into(),
            );
            let edges: Vec<String> = derived
                .induced
                .get(rel)
                .map(|v| {
                    v.iter()
                        .map(|e| {
                            format!(
                                "{} ({})",
                                e.strategy,
                                e.raw_class.as_deref().unwrap_or("no class")
                            )
                        })
                        .collect()
                })
                .unwrap_or_default();
            put("induced_by", edges.into());
        }
        Kind::Strategy => {
            put(
                "band",
                derived.band(rel).map(|b| b.as_str().to_string()).into(),
            );
        }
        Kind::Plan => {
            if let Some(fm) = &a.fm {
                put("type", fm.get_str("type").into());
                put("complexity", fm.get_str("complexity").into());
                put("awaits", fm.get_list("awaits").unwrap_or_default().into());
                put("handoff", fm.get_str("handoff").into());
            }
            put(
                "effective_class",
                derived
                    .plan_class(tree, rel)
                    .map(|c| c.as_str().to_string())
                    .into(),
            );
            if let Some((target, status)) = derived.hold(rel) {
                put(
                    "held",
                    format!(
                        "awaits {target} (status: {})",
                        status
                            .map(|s| s.as_str().to_string())
                            .unwrap_or_else(|| "missing".into())
                    )
                    .into(),
                );
            }
            put("open_escalations", open_escalations(tree, rel).into());
        }
        _ => {}
    }
    Some(serde_json::Value::Object(obj))
}

/// One row of the artifact census.
#[derive(Debug, Serialize)]
pub struct ArtifactRow {
    pub path: String,
    pub kind: String,
    pub provenance: Option<String>,
    pub owner: Option<String>,
    pub status: Option<String>,
}

/// `trellis tree`: the artifacts, and — inseparably — what discovery left out.
/// A census that showed only what it found would answer half the question.
#[derive(Debug, Serialize)]
pub struct TreeReport {
    pub version: u32,
    pub root: String,
    pub artifacts: Vec<ArtifactRow>,
    pub scope: Scope,
}

/// Every artifact discovery took in, sorted by path — the answer to "what does
/// the kernel consider mine?", which is the question behind a lint finding on a
/// file nobody here wrote.
pub fn artifact_rows(tree: &Tree) -> Vec<ArtifactRow> {
    let mut rows: Vec<ArtifactRow> = tree
        .artifacts
        .iter()
        .map(|a| ArtifactRow {
            path: a.rel.clone(),
            kind: kind_name(a.kind),
            provenance: a.provenance(),
            owner: a.owner(),
            status: a.status(),
        })
        .collect();
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    rows
}

/// One row of the plan census.
#[derive(Debug, Serialize)]
pub struct PlanRow {
    pub plan: String,
    /// The body's first `#` heading — how the plan names itself, where the
    /// path is only where it lives.
    pub title: Option<String>,
    pub status: Option<String>,
    pub r#type: Option<String>,
    pub owner: Option<String>,
    pub complexity: Option<String>,
    /// The `awaits:` target holding this plan, when one does.
    pub held: Option<String>,
    /// What an active plan is parked on, when its taker declared one — a
    /// handoff is why a plan can sit `active` with no session on it.
    pub handoff: Option<String>,
    /// Every `awaits:` target, held or not — the plan's outgoing edges in
    /// the sequencing DAG, so a caller can draw the graph from the census
    /// alone. Targets are live addresses (spec rule 13).
    pub awaits: Vec<String>,
    pub dwell_days: Option<i64>,
    pub open_escalations: usize,
    /// Filed in the terminal tier. Rows carry it rather than being dropped
    /// here, so a caller can ask for the whole census when it wants one.
    pub archived: bool,
}

/// Cut a census down to the live one: drop the rows filed in the terminal
/// tier, and drop the edges that named them with them. The tier leaves
/// attention (spec rule 13), and an `awaits:` target that has been filed is a
/// hold that already cleared, so the edge leaves with the row it named — the
/// same drop `view plan-graph` performs on a retired target.
///
/// The prune is keyed on the archived rows, never on a target failing to
/// resolve. A target matching no row at all is the other thing entirely — a
/// ref naming no plan, which lint item 4 owns and a graph draws as a hole
/// rather than silently dropping.
pub fn live_only(rows: &mut Vec<PlanRow>) {
    let filed: HashSet<String> = rows
        .iter()
        .filter(|r| r.archived)
        .map(|r| crate::tree::live_path(&r.plan).to_string())
        .collect();
    rows.retain(|r| !r.archived);
    for row in rows {
        row.awaits
            .retain(|t| !filed.contains(crate::tree::live_path(t.trim())));
    }
}

/// `trellis plan list`: the whole plan census, sorted by path. Callers apply
/// their own `--status` / `--held` cuts over the rows.
pub fn plan_rows(tree: &Tree, git: &Git, derived: &Derived, today: Date) -> Vec<PlanRow> {
    let mut plans: Vec<_> = tree.by_kind(Kind::Plan).collect();
    plans.sort_by(|a, b| a.rel.cmp(&b.rel));
    plans
        .into_iter()
        .map(|p| {
            let status = p.status();
            let dwell = status.as_ref().and_then(|s| {
                git.status_set_date(&p.rel, s)
                    .map(|d| dates::days_between(d, today))
            });
            PlanRow {
                title: p
                    .headings
                    .iter()
                    .find(|h| h.level == 1)
                    .map(|h| h.text.clone()),
                r#type: p.fm.as_ref().and_then(|f| f.get_str("type")),
                owner: p.owner(),
                complexity: p.fm.as_ref().and_then(|f| f.get_str("complexity")),
                held: derived.hold(&p.rel).map(|(t, _)| t),
                handoff: p.fm.as_ref().and_then(|f| f.get_str("handoff")),
                awaits: derived
                    .plan_awaits
                    .get(crate::tree::live_path(&p.rel))
                    .cloned()
                    .unwrap_or_default(),
                dwell_days: dwell,
                open_escalations: escalation_records(p)
                    .iter()
                    .filter(|r| r.status.as_deref() == Some("open"))
                    .count(),
                status,
                archived: p.archived,
                plan: p.rel.clone(),
            }
        })
        .collect()
}
