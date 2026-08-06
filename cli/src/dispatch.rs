//! The plan-dispatch scan. Deterministic by construction — declared-field
//! reads only, no judgment. Originally an awk brain inside a forge workflow,
//! ported here whole by decision 0037 and now run in-process by the daemon
//! (0039). The complexity→session mapping ships as defaults but stays the
//! *binding's* tuning point, through `--map` or `runtime.toml`'s `[sessions]`
//! (decision 0032: retuned in the binding, never in a plan).

use serde::Serialize;

use crate::model::{Complexity, PlanStatus};
use crate::tree::{Kind, Tree};

#[derive(Debug, Clone, Serialize)]
pub struct DispatchItem {
    pub plan: String,
    pub owner: String,
    pub owner_short: String,
    pub complexity: String,
    pub model: String,
    pub effort: String,
    pub budget_usd: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct HeldItem {
    pub plan: String,
    pub awaits: String,
    /// The holding target's status; `None` when the target does not resolve.
    pub target_status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub version: u32,
    pub dispatch: Vec<DispatchItem>,
    pub held: Vec<HeldItem>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub model: String,
    pub effort: String,
    pub budget_usd: f64,
}

#[derive(Debug, Clone)]
pub struct SessionMap {
    pub mechanical: Session,
    pub standard: Session,
    pub deep: Session,
}

impl Default for SessionMap {
    fn default() -> Self {
        let s = |model: &str, effort: &str, budget: f64| Session {
            model: model.into(),
            effort: effort.into(),
            budget_usd: budget,
        };
        SessionMap {
            mechanical: s("opus", "high", 5.0),
            standard: s("opus", "xhigh", 10.0),
            deep: s("fable", "xhigh", 25.0),
        }
    }
}

impl SessionMap {
    /// Apply a `tier=model:effort:budget` override.
    pub fn apply(&mut self, spec: &str) -> anyhow::Result<()> {
        let (tier, rest) = spec
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("--map expects tier=model:effort:budget, got {spec}"))?;
        let parts: Vec<&str> = rest.split(':').collect();
        let [model, effort, budget] = parts.as_slice() else {
            anyhow::bail!("--map expects tier=model:effort:budget, got {spec}");
        };
        let session = Session {
            model: model.to_string(),
            effort: effort.to_string(),
            budget_usd: budget.parse()?,
        };
        match tier {
            "mechanical" => self.mechanical = session,
            "standard" => self.standard = session,
            "deep" => self.deep = session,
            other => {
                anyhow::bail!("{other} is not a complexity tier (mechanical | standard | deep)")
            }
        }
        Ok(())
    }

    fn session(&self, c: Complexity) -> &Session {
        match c {
            Complexity::Mechanical => &self.mechanical,
            Complexity::Standard => &self.standard,
            Complexity::Deep => &self.deep,
        }
    }
}

pub fn scan(tree: &Tree, map: &SessionMap) -> ScanReport {
    let mut report = ScanReport {
        version: 1,
        dispatch: vec![],
        held: vec![],
        warnings: vec![],
    };

    // The terminal tier never carries work. Archived plans are terminal by
    // admission, so none could be `ready` anyway — skipping them keeps the
    // scan off a set that only grows, on a path that runs every tick.
    let mut plans: Vec<_> = tree.by_kind(Kind::Plan).filter(|p| !p.archived).collect();
    plans.sort_by(|a, b| a.rel.cmp(&b.rel));

    'plans: for plan in plans {
        let Some(fm) = &plan.fm else {
            // The awk read an empty status and skipped silently; we skip
            // with a warning — malformed frontmatter is never dispatched.
            report.warnings.push(format!(
                "{} has no readable frontmatter — skipping",
                plan.rel
            ));
            continue;
        };
        if fm.get_str("status").as_deref() != Some(PlanStatus::Ready.as_str()) {
            continue;
        }

        for target in fm.get_list("awaits").unwrap_or_default() {
            match tree.get(&target) {
                None => {
                    report.warnings.push(format!(
                        "{} awaits '{}', which does not resolve — holding it in the queue (conventions-lint item 4)",
                        plan.rel, target
                    ));
                    report.held.push(HeldItem {
                        plan: plan.rel.clone(),
                        awaits: target,
                        target_status: None,
                    });
                    continue 'plans;
                }
                Some(t) => {
                    let status = t.status();
                    if status.as_deref() != Some(PlanStatus::Retired.as_str()) {
                        report.held.push(HeldItem {
                            plan: plan.rel.clone(),
                            awaits: target,
                            target_status: status,
                        });
                        continue 'plans;
                    }
                }
            }
        }

        let Some(owner) = fm.get_str("owner") else {
            report.warnings.push(format!(
                "{} is ready but declares no owner — skipping",
                plan.rel
            ));
            continue;
        };

        let raw = fm.get_str("complexity");
        let complexity = match raw.as_deref() {
            None => Complexity::Standard,
            Some(c) => match Complexity::parse(c) {
                Some(c) => c,
                None => {
                    report.warnings.push(format!(
                        "{} declares complexity '{c}', which is not a legal tier — dispatching with the standard session (conventions-lint item 4)",
                        plan.rel
                    ));
                    Complexity::Standard
                }
            },
        };
        let session = map.session(complexity);
        report.dispatch.push(DispatchItem {
            plan: plan.rel.clone(),
            owner_short: owner.strip_prefix("org/").unwrap_or(&owner).to_string(),
            owner,
            complexity: complexity.as_str().to_string(),
            model: session.model.clone(),
            effort: session.effort.clone(),
            budget_usd: session.budget_usd,
        });
    }

    report
}
