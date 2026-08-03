//! Guarded plan lifecycle transitions. The flip itself is a one-line
//! frontmatter edit; the guard refuses illegal source states so an agent
//! cannot claim a draft or re-release a retired plan. Who may flip is the
//! invoker's mandate, not the tool's business.

use std::path::Path;

use anyhow::bail;

use crate::fmedit;
use crate::model::PlanStatus;

pub fn current_status(path: &Path) -> anyhow::Result<PlanStatus> {
    let raw = fmedit::get(path, "status")?
        .ok_or_else(|| anyhow::anyhow!("{} declares no status:", path.display()))?;
    PlanStatus::parse(&raw)
        .ok_or_else(|| anyhow::anyhow!("{} has illegal status: {raw}", path.display()))
}

/// Flip `status:` after checking the source state is legal for the move.
pub fn flip(path: &Path, from: &[PlanStatus], to: PlanStatus) -> anyhow::Result<PlanStatus> {
    let current = current_status(path)?;
    if current == to {
        bail!("{} is already {}", path.display(), to.as_str());
    }
    if !from.contains(&current) {
        bail!(
            "{} is {} — this transition starts from {}",
            path.display(),
            current.as_str(),
            from.iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(" | ")
        );
    }
    fmedit::set_scalar(path, "status", to.as_str(), false)?;
    Ok(current)
}
