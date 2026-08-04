//! Computed facts in the shapes the commands already print. Extracted from
//! `main.rs` so a second consumer — the daemon's HTTP surface — serves the
//! same JSON a command does. One shape per fact; a parallel encoding is the
//! drift decision 0037 exists to prevent.

use serde::Serialize;

use crate::dates::{self, Date};
use crate::gitio::Git;
use crate::graph::Derived;
use crate::model::escalation_records;
use crate::tree::{Kind, Tree};

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
    put("kind", format!("{:?}", a.kind).to_lowercase().into());
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

/// One row of the plan census.
#[derive(Debug, Serialize)]
pub struct PlanRow {
    pub plan: String,
    pub status: Option<String>,
    pub r#type: Option<String>,
    pub owner: Option<String>,
    pub complexity: Option<String>,
    /// The `awaits:` target holding this plan, when one does.
    pub held: Option<String>,
    pub dwell_days: Option<i64>,
    pub open_escalations: usize,
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
                r#type: p.fm.as_ref().and_then(|f| f.get_str("type")),
                owner: p.owner(),
                complexity: p.fm.as_ref().and_then(|f| f.get_str("complexity")),
                held: derived.hold(&p.rel).map(|(t, _)| t),
                dwell_days: dwell,
                open_escalations: escalation_records(p)
                    .iter()
                    .filter(|r| r.status.as_deref() == Some("open"))
                    .count(),
                status,
                plan: p.rel.clone(),
            }
        })
        .collect()
}
