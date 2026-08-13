//! The mechanical share of `checks/plan-readiness.md`, with honesty tiers:
//! items 3 and 4 are fully mechanical, 5 and 10 are partial (their remainder
//! is stated), 7 is a heuristic token scan, and the rest are judgment.
//! A plan that fails the mechanical share is not releasable; passing it says
//! nothing about the judgment items — the output names what remains.

use serde::Serialize;

use crate::graph::Derived;
use crate::refs;
use crate::tree::Tree;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemStatus {
    Pass,
    Fail,
    Partial,
    Judgment,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReadinessItem {
    pub item: u8,
    pub title: &'static str,
    pub status: ItemStatus,
    pub detail: String,
}

#[derive(Debug, Serialize)]
pub struct ReadinessReport {
    pub plan: String,
    pub items: Vec<ReadinessItem>,
    /// Every mechanical (and partial-mechanical) item passed.
    pub mechanical_pass: bool,
}

const DEFERRAL_TOKENS: &[&str] = &[
    "tbd",
    "to be determined",
    "to be decided",
    "decide later",
    "pick one of",
    "placeholder",
    "fixme",
    "???",
];

pub fn check(tree: &Tree, derived: &Derived, plan_rel: &str) -> anyhow::Result<ReadinessReport> {
    let plan = tree
        .get(plan_rel)
        .ok_or_else(|| anyhow::anyhow!("{plan_rel} does not exist in this root"))?;
    let fm = plan
        .fm
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("{plan_rel} has no frontmatter"))?;

    let mut items = Vec::new();
    let judgment = |item, title, detail: &str| ReadinessItem {
        item,
        title,
        status: ItemStatus::Judgment,
        detail: detail.to_string(),
    };

    items.push(judgment(
        1,
        "objective is an outcome, not a task list",
        "judgment — read the objective",
    ));
    items.push(judgment(
        2,
        "done criterion is self-evaluable",
        "judgment — read the measurement",
    ));

    // 3. Declared refs resolve.
    {
        let mut failures = Vec::new();
        for key in ["subdomains", "contexts", "metrics", "decisions", "awaits"] {
            for r in fm.get_list(key).unwrap_or_default() {
                if let Err(reason) = refs::resolve(&r, tree) {
                    failures.push(format!("{key}: {r} — {reason}"));
                }
            }
        }
        items.push(ReadinessItem {
            item: 3,
            title: "declared refs resolve",
            status: if failures.is_empty() {
                ItemStatus::Pass
            } else {
                ItemStatus::Fail
            },
            detail: if failures.is_empty() {
                "all refs resolve".into()
            } else {
                failures.join("; ")
            },
        });
    }

    // 4. Change mechanics determinable: the effective automation class is
    // computable from the plan's subdomains.
    {
        let subs = fm.get_list("subdomains").unwrap_or_default();
        let (status, detail) = if subs.is_empty() {
            (
                ItemStatus::Fail,
                "plan declares no subdomains: — the effective automation class is not computable"
                    .to_string(),
            )
        } else {
            match derived.plan_class(tree, plan_rel) {
                Some(class) => (ItemStatus::Pass, format!("effective automation class: {}", class.as_str())),
                None => (ItemStatus::Fail, "no subdomains: ref resolves to a subdomain with induced-by edges — class not computable".to_string()),
            }
        };
        items.push(ReadinessItem {
            item: 4,
            title: "change mechanics determinable",
            status,
            detail,
        });
    }

    // 5. Owner is real; its mandate scope covers the plan's subdomains
    // (mechanical only when scope entries are refs).
    {
        let (status, detail) = match fm.get_str("owner") {
            None => (ItemStatus::Fail, "plan declares no owner:".to_string()),
            Some(owner) if !tree.role_exists(&owner) => (
                ItemStatus::Fail,
                format!("owner: {owner} does not resolve to a role in this root"),
            ),
            Some(owner) => {
                let role = owner.strip_prefix("org/").unwrap_or(&owner);
                let mandate = tree.get(&format!("org/{role}/mandate.md"));
                let scope: Vec<String> = mandate
                    .and_then(|m| m.fm.as_ref())
                    .and_then(|fm| fm.get_list("scope"))
                    .unwrap_or_default();
                let subs = fm.get_list("subdomains").unwrap_or_default();
                let scope_is_refs = !scope.is_empty()
                    && scope
                        .iter()
                        .all(|s| matches!(refs::classify(s), refs::RefKind::Path { .. }));
                if scope_is_refs {
                    let uncovered: Vec<&String> =
                        subs.iter().filter(|s| !scope.contains(s)).collect();
                    if uncovered.is_empty() {
                        (
                            ItemStatus::Pass,
                            format!("{owner} exists; scope covers the plan's subdomains"),
                        )
                    } else {
                        (
                            ItemStatus::Fail,
                            format!(
                                "{} outside {owner}'s mandate scope",
                                uncovered
                                    .iter()
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ),
                        )
                    }
                } else {
                    (
                        ItemStatus::Partial,
                        format!(
                            "{owner} exists; its scope is prose ({}) — coverage is a judgment read",
                            if scope.is_empty() {
                                "empty".to_string()
                            } else {
                                scope.join(", ")
                            }
                        ),
                    )
                }
            }
        };
        items.push(ReadinessItem {
            item: 5,
            title: "owner real, scope covers",
            status,
            detail,
        });
    }

    items.push(judgment(
        6,
        "required authority inside authority: or named as escalation",
        "judgment — compare the plan's moves against the mandate",
    ));

    // 7. No unresolved deferrals — read over the body *above* `## Escalations`.
    //
    // An escalation record quotes the deferral it was raised about; that is
    // what raising one means. Records are kept verbatim and never deleted
    // (`conventions.md`, "Escalation records"), so a domain that escalates
    // about an unset done criterion can never clear this item by any edit the
    // conventions permit: resolving the escalation answers the question and
    // leaves its quotation behind, and since decision 0045 the same scan gates
    // dispatch — so the plan is held forever for having reported the defect it
    // then fixed. Item 7 reads the plan's instructions to a taker. The trail is
    // not instructions.
    {
        let mut hits = Vec::new();
        let tbd_word = regex::Regex::new(r"\btbd\b").unwrap();
        let escalations = regex::Regex::new(r"(?i)^#{1,6}\s+escalations\s*$").unwrap();
        let body = plan.body();
        let total = body.lines().count();
        let scanned = body
            .lines()
            .enumerate()
            .take_while(|(_, line)| !escalations.is_match(line));
        let mut read = 0usize;

        for (i, line) in scanned {
            read += 1;
            let lower = line.to_lowercase();
            for token in DEFERRAL_TOKENS {
                if lower.contains(token) {
                    // "tbd" only as a word, not inside another token.
                    if *token == "tbd" && !tbd_word.is_match(&lower) {
                        continue;
                    }
                    hits.push(format!("line {}: {}", i + 1, line.trim()));
                    break;
                }
            }
        }

        items.push(ReadinessItem {
            item: 7,
            title: "no unresolved deferrals",
            status: if hits.is_empty() {
                ItemStatus::Pass
            } else {
                ItemStatus::Fail
            },
            detail: if !hits.is_empty() {
                hits.join("; ")
            } else if read < total {
                "no deferral tokens found above ## Escalations".into()
            } else {
                "no deferral tokens found".into()
            },
        });
    }

    items.push(judgment(
        8,
        "decisions it rests on are recorded",
        "judgment — the plan's premises against decisions/",
    ));
    items.push(judgment(
        9,
        "underdetermination test (agent takers)",
        "judgment — could two reasonable takers ship different things?",
    ));

    // 10. Target reachable and writable — mechanical half: declared contexts
    // exist in the working tree.
    {
        let contexts = fm.get_list("contexts").unwrap_or_default();
        let missing: Vec<&String> = contexts
            .iter()
            .filter(|c| !tree.has_dir(c.trim_end_matches('/')))
            .collect();
        let (status, detail) = if contexts.is_empty() {
            (ItemStatus::Partial, "plan declares no contexts: — nothing to check mechanically; writability is a judgment read".to_string())
        } else if missing.is_empty() {
            (ItemStatus::Partial, "declared contexts exist; read-only/imported status is judged against the instance's boundary guarantees".to_string())
        } else {
            (
                ItemStatus::Fail,
                format!(
                    "contexts do not exist: {}",
                    missing
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            )
        };
        items.push(ReadinessItem {
            item: 10,
            title: "target reachable and writable",
            status,
            detail,
        });
    }

    items.push(judgment(
        11,
        "verification exists",
        "judgment — tests, a contracts/ artifact, an eval, or a stated manual check",
    ));

    let mechanical_pass = items.iter().all(|i| i.status != ItemStatus::Fail);
    Ok(ReadinessReport {
        plan: plan_rel.to_string(),
        items,
        mechanical_pass,
    })
}
