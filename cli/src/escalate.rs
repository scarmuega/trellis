//! Escalation records: fenced `yaml` blocks under `## Escalations` in the
//! artifact the escalation concerns (decision 0036). The kernel scaffolds a
//! well-formed record and flips `open → resolved`; the prose — the ask, the
//! answer — stays authored content.

use anyhow::bail;
use serde::Serialize;

use crate::dates::Date;
use crate::fmedit;
use crate::model::{escalation_records, EscalationRecord};
use crate::tree::Tree;

#[derive(Debug)]
pub struct NewEscalation {
    pub by: String,
    pub to: Option<String>,
    pub asks: String,
    pub attempted: Option<String>,
    pub blocked: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListedRecord {
    pub artifact: String,
    pub line: u32,
    pub status: Option<String>,
    pub raised: Option<String>,
    pub by: Option<String>,
    pub to: Option<String>,
    pub asks: Option<String>,
}

/// Append a record to the artifact. `to:` defaults to the raiser's
/// `escalate-to:`, falling back to the artifact's owner (the schema's rule).
pub fn add(
    tree: &Tree,
    artifact_rel: &str,
    e: &NewEscalation,
    today: Date,
) -> anyhow::Result<String> {
    let artifact = tree
        .get(artifact_rel)
        .ok_or_else(|| anyhow::anyhow!("{artifact_rel} does not exist in this root"))?;
    if artifact.provenance().as_deref() == Some("generated") {
        bail!("{artifact_rel} is generated — a generated artifact is never annotated; the escalation goes to its generator's source");
    }
    if !tree.role_exists(&e.by) {
        bail!("--by {} does not resolve to a role in this root", e.by);
    }
    let to = match &e.to {
        Some(t) => t.clone(),
        None => {
            let role = e.by.strip_prefix("org/").unwrap_or(&e.by);
            tree.get(&format!("org/{role}/mandate.md"))
                .and_then(|m| m.fm.as_ref())
                .and_then(|fm| fm.get_str("escalate-to"))
                .or_else(|| artifact.owner())
                .ok_or_else(|| anyhow::anyhow!("cannot derive to: — {} has no escalate-to: and {artifact_rel} has no owner; pass --to", e.by))?
        }
    };
    if !tree.role_exists(&to) {
        bail!("to: {to} does not resolve to a role in this root");
    }

    let mut block = String::from("```yaml\n");
    block.push_str(&format!("raised: {today}\n"));
    block.push_str(&format!("by: {}\n", e.by));
    block.push_str(&format!("to: {to}\n"));
    block.push_str("status: open\n");
    block.push_str(&format!("asks: {}\n", e.asks));
    if let Some(a) = &e.attempted {
        block.push_str(&format!("attempted: {a}\n"));
    }
    if let Some(b) = &e.blocked {
        block.push_str(&format!("blocked: {b}\n"));
    }
    block.push_str("```");

    fmedit::append_to_section(&artifact.abs, "Escalations", &block)?;
    Ok(to)
}

pub fn list(tree: &Tree, include_resolved: bool) -> Vec<ListedRecord> {
    let mut out = Vec::new();
    for a in &tree.artifacts {
        for r in escalation_records(a) {
            if !include_resolved && r.status.as_deref() != Some("open") {
                continue;
            }
            out.push(ListedRecord {
                artifact: a.rel.clone(),
                line: r.line,
                status: r.status,
                raised: r.raised,
                by: r.by,
                to: r.to,
                asks: r.asks,
            });
        }
    }
    out.sort_by(|a, b| {
        (
            a.raised.as_deref().unwrap_or(""),
            a.artifact.as_str(),
            a.line,
        )
            .cmp(&(
                b.raised.as_deref().unwrap_or(""),
                b.artifact.as_str(),
                b.line,
            ))
    });
    out
}

/// Flip one open record to resolved. Selector: `raised:` date when given;
/// otherwise the artifact must carry exactly one open record.
pub fn resolve(
    tree: &Tree,
    artifact_rel: &str,
    raised: Option<&str>,
) -> anyhow::Result<EscalationRecord> {
    let artifact = tree
        .get(artifact_rel)
        .ok_or_else(|| anyhow::anyhow!("{artifact_rel} does not exist in this root"))?;
    let open: Vec<EscalationRecord> = escalation_records(artifact)
        .into_iter()
        .filter(|r| r.status.as_deref() == Some("open"))
        .filter(|r| {
            raised
                .map(|d| r.raised.as_deref() == Some(d))
                .unwrap_or(true)
        })
        .collect();
    let record = match open.len() {
        0 => bail!(
            "{artifact_rel} has no open escalation record{}",
            raised.map(|d| format!(" raised {d}")).unwrap_or_default()
        ),
        1 => open.into_iter().next().unwrap(),
        n => bail!("{artifact_rel} has {n} open records — pass --raised YYYY-MM-DD to pick one"),
    };

    // Rewrite the `status: open` line inside that record's fence.
    let text = std::fs::read_to_string(&artifact.abs)?;
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let start = record.line as usize; // fence opener is 1-based; content starts next line
    let mut done = false;
    for line in lines.iter_mut().skip(start) {
        if line.trim_start().starts_with("```") {
            break;
        }
        if line
            .strip_prefix("status:")
            .map(|v| v.trim() == "open")
            .unwrap_or(false)
        {
            *line = "status: resolved".to_string();
            done = true;
            break;
        }
    }
    if !done {
        bail!("could not locate the record's status: open line — resolve it by hand");
    }
    let ending = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut rendered = lines.join(ending);
    if text.ends_with('\n') {
        rendered.push_str(ending);
    }
    let dir = artifact
        .abs
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(dir)?;
    std::fs::write(tmp.path(), &rendered)?;
    tmp.persist(&artifact.abs)
        .map_err(|e| anyhow::anyhow!("atomic rename failed: {e}"))?;
    Ok(record)
}
