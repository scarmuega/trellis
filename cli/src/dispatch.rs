//! The plan-dispatch scan. Deterministic by construction — declared-field
//! reads only, no judgment. Originally an awk brain inside a forge workflow,
//! ported here whole by decision 0037 and now run in-process by the daemon
//! (0039). The complexity→session mapping ships as defaults but stays the
//! *binding's* tuning point, through `--map` or `runtime.toml`'s `[sessions]`
//! (decision 0032: retuned in the binding, never in a plan).

use serde::Serialize;

use crate::graph::Derived;
use crate::model::{Complexity, PlanStatus};
use crate::org;
use crate::readiness;
use crate::tree::{Kind, Tree};

#[derive(Debug, Clone, Serialize)]
pub struct DispatchItem {
    pub plan: String,
    pub owner: String,
    pub owner_short: String,
    /// The owner mandate's `escalate-to:`, or the owner itself when the
    /// mandate is silent — a declared field the session would otherwise
    /// spend a read deriving (decision 0045).
    pub escalate_to: String,
    pub complexity: String,
    pub model: String,
    pub effort: String,
    pub budget_usd: f64,
}

/// Where a role's escalations go: its mandate's `escalate-to:`, falling back
/// to the role itself (an escalation to yourself is a report that stands).
pub fn escalate_to(tree: &Tree, role: &str) -> String {
    let short = role.strip_prefix("org/").unwrap_or(role);
    tree.get(&format!("org/{short}/mandate.md"))
        .and_then(|m| m.fm.as_ref())
        .and_then(|f| f.get_str("escalate-to"))
        .unwrap_or_else(|| role.to_string())
}

/// Why a `ready` plan is skipped this pass. Both reasons hold, never block:
/// the plan stays `ready`, nothing is written, and the hold clears itself —
/// retiring the target, or fixing what the mechanical items name.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "reason", rename_all = "lowercase")]
pub enum HoldReason {
    /// Declared sequencing: an `awaits:` target is not `retired`.
    Awaits {
        awaits: String,
        /// The holding target's status; `None` when the target does not
        /// resolve.
        target_status: Option<String>,
    },
    /// The mechanical share of the readiness gate failed — dispatching would
    /// spawn a session whose whole work product is discovering this.
    Readiness {
        failed_items: Vec<u8>,
        detail: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct HeldItem {
    pub plan: String,
    #[serde(flatten)]
    pub reason: HoldReason,
}

impl HeldItem {
    /// The hold as one log-line clause, reason-shaped.
    pub fn describe(&self) -> String {
        match &self.reason {
            HoldReason::Awaits {
                awaits,
                target_status,
            } => format!(
                "awaits {awaits} (status: {})",
                target_status.as_deref().unwrap_or("unresolved")
            ),
            HoldReason::Readiness {
                failed_items,
                detail,
            } => format!(
                "fails mechanical readiness item(s) {} — {detail}",
                failed_items
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

/// A `ready` plan whose owner's holder is a declared human: no session to
/// spawn — the work is a handoff, and the scan's job ends at saying so.
#[derive(Debug, Clone, Serialize)]
pub struct HandoffItem {
    pub plan: String,
    pub owner: String,
    pub holder_ref: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ScanReport {
    pub version: u32,
    pub dispatch: Vec<DispatchItem>,
    pub held: Vec<HeldItem>,
    pub handoffs: Vec<HandoffItem>,
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
            mechanical: s("opus", "medium", 5.0),
            standard: s("opus", "high", 10.0),
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

pub fn scan(tree: &Tree, derived: &Derived, map: &SessionMap) -> ScanReport {
    let mut report = ScanReport {
        version: 2,
        dispatch: vec![],
        held: vec![],
        handoffs: vec![],
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
                        reason: HoldReason::Awaits {
                            awaits: target,
                            target_status: None,
                        },
                    });
                    continue 'plans;
                }
                Some(t) => {
                    let status = t.status();
                    if status.as_deref() != Some(PlanStatus::Retired.as_str()) {
                        report.held.push(HeldItem {
                            plan: plan.rel.clone(),
                            reason: HoldReason::Awaits {
                                awaits: target,
                                target_status: status,
                            },
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
        let owner_short = owner.strip_prefix("org/").unwrap_or(&owner).to_string();

        // The mechanical share of the readiness gate, before a session is
        // spent discovering it (decision 0045). Declared-field reads and the
        // deterministic checks readiness.rs already tiers as mechanical —
        // the judgment items stay the session's. A failure holds; blocking
        // and the escalation record are a mandated session's acts, never the
        // scan's (spec runtime.md, "Plan dispatch" and its limitations).
        match readiness::check(tree, derived, &plan.rel) {
            Ok(r) if !r.mechanical_pass => {
                let failed: Vec<&readiness::ReadinessItem> = r
                    .items
                    .iter()
                    .filter(|i| i.status == readiness::ItemStatus::Fail)
                    .collect();
                report.held.push(HeldItem {
                    plan: plan.rel.clone(),
                    reason: HoldReason::Readiness {
                        failed_items: failed.iter().map(|i| i.item).collect(),
                        detail: failed
                            .iter()
                            .map(|i| format!("{}: {}", i.title, i.detail))
                            .collect::<Vec<_>>()
                            .join("; "),
                    },
                });
                continue;
            }
            Ok(_) => {}
            Err(e) => {
                report.warnings.push(format!(
                    "{}: readiness precheck errored ({e:#}) — dispatching anyway",
                    plan.rel
                ));
            }
        }

        // A declared-human holder gets a handoff, not a session: the scan
        // reads one declared field (`kind:` on holder/ref.md) and stops. An
        // undeclared holder keeps the pre-0045 behavior — spawn, and let the
        // session apply act's never-impersonate rule.
        let holder = org::holder(tree, &owner_short);
        if holder.is_declared_human() {
            report.handoffs.push(HandoffItem {
                plan: plan.rel.clone(),
                owner,
                holder_ref: holder.reference,
            });
            continue;
        }

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
            escalate_to: escalate_to(tree, &owner),
            owner_short,
            owner,
            complexity: complexity.as_str().to_string(),
            model: session.model.clone(),
            effort: session.effort.clone(),
            budget_usd: session.budget_usd,
        });
    }

    report
}
