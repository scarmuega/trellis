//! Derived values the spec defines over the artifact graph: strategy bands,
//! effective automation class, orphans (structural and economic), the
//! capture point, and the `awaits:` DAG. Computed once per run; every agent
//! used to re-derive these per session.

use std::collections::{HashMap, HashSet};

use crate::model::{Band, Class, PlanStatus, StrategyStatus};
use crate::tree::{Kind, Tree};

#[derive(Debug, Clone)]
pub struct InducedEdge {
    pub strategy: String,
    pub class: Option<Class>,
    pub raw_class: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FundEdge {
    pub target: String,
    /// `current` when absent (the schema's default).
    pub current: bool,
    pub raw_relation: Option<String>,
}

#[derive(Debug, Default)]
pub struct Derived {
    pub strategy_status: HashMap<String, Option<StrategyStatus>>,
    pub induced: HashMap<String, Vec<InducedEdge>>,
    pub funded: HashMap<String, Vec<FundEdge>>,
    pub plan_status: HashMap<String, Option<PlanStatus>>,
    pub plan_awaits: HashMap<String, Vec<String>>,
    /// Accepted successor → the decisions it supersedes, resolved (globs
    /// expanded when they match exactly one). The successor carries the edge
    /// (spec rule 6): the superseded file never changes, so liveness is
    /// derived here, never declared on the target. Only accepted successors
    /// enter — a draft supersedes nothing yet.
    pub decision_supersedes: HashMap<String, Vec<String>>,
}

pub fn derive(tree: &Tree) -> Derived {
    let mut d = Derived::default();

    for a in tree.by_kind(Kind::Strategy) {
        let status = a.status().and_then(|s| StrategyStatus::parse(&s));
        d.strategy_status.insert(a.rel.clone(), status);
        let edges =
            a.fm.as_ref()
                .and_then(|fm| fm.get_edges("funded-by", "strategy"))
                .unwrap_or_default()
                .into_iter()
                .map(|e| FundEdge {
                    target: e.target,
                    current: e
                        .relation
                        .as_deref()
                        .map(|r| r == "current")
                        .unwrap_or(true),
                    raw_relation: e.relation,
                })
                .collect();
        d.funded.insert(a.rel.clone(), edges);
    }

    for a in tree.by_kind(Kind::Problem) {
        let edges =
            a.fm.as_ref()
                .and_then(|fm| fm.get_edges("induced-by", "strategy"))
                .unwrap_or_default()
                .into_iter()
                .map(|e| InducedEdge {
                    strategy: e.target,
                    class: e.class.as_deref().and_then(Class::parse),
                    raw_class: e.class,
                })
                .collect();
        d.induced.insert(a.rel.clone(), edges);
    }

    // Decisions never move (rule 13), so their rel is their address. An
    // ill-formed edge — self, non-decision, unresolvable, ambiguous glob —
    // stays out of the map (the target stays live, fail closed) and lint
    // item 26 owns the complaint.
    let decision_rels: Vec<String> = tree
        .by_kind(Kind::Decision)
        .map(|a| a.rel.clone())
        .collect();
    for a in tree.by_kind(Kind::Decision) {
        if a.status().as_deref() != Some("accepted") {
            continue;
        }
        let raw =
            a.fm.as_ref()
                .and_then(|fm| fm.get_list("supersedes"))
                .unwrap_or_default();
        let mut targets: Vec<String> = Vec::new();
        for r in raw {
            let r = r.trim().to_string();
            if r == a.rel || !r.starts_with("decisions/") {
                continue;
            }
            if let Some(prefix) = r.split_once('*').map(|(p, _)| p) {
                let mut hits: Vec<&String> = decision_rels
                    .iter()
                    .filter(|d| d.starts_with(prefix) && **d != a.rel)
                    .collect();
                hits.sort();
                if let [only] = hits.as_slice() {
                    targets.push((*only).clone());
                }
            } else if decision_rels.contains(&r) {
                targets.push(r);
            }
        }
        if !targets.is_empty() {
            targets.sort();
            targets.dedup();
            d.decision_supersedes.insert(a.rel.clone(), targets);
        }
    }

    // Keyed by *address*, not location: an archived plan is still named by
    // its live path, so an `awaits:` edge written before the move keeps
    // resolving and its hold still clears (spec rule 13).
    for a in tree.by_kind(Kind::Plan) {
        d.plan_status.insert(
            crate::tree::live_path(&a.rel).to_string(),
            a.status().and_then(|s| PlanStatus::parse(&s)),
        );
        d.plan_awaits.insert(
            crate::tree::live_path(&a.rel).to_string(),
            a.fm.as_ref()
                .and_then(|fm| fm.get_list("awaits"))
                .unwrap_or_default(),
        );
    }

    d
}

impl Derived {
    pub fn band(&self, strategy: &str) -> Option<Band> {
        self.strategy_status.get(strategy)?.map(|s| s.band())
    }

    pub fn committed(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .strategy_status
            .iter()
            .filter(|(_, s)| s.map(|s| s.band() == Band::Committed).unwrap_or(false))
            .map(|(k, _)| k.as_str())
            .collect();
        v.sort();
        v
    }

    /// Edges from a subdomain to committed-band strategies.
    pub fn committed_edges(&self, subdomain: &str) -> Vec<&InducedEdge> {
        self.induced
            .get(subdomain)
            .map(|edges| {
                edges
                    .iter()
                    .filter(|e| self.band(&e.strategy) == Some(Band::Committed))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Effective automation class: strictest class across edges to
    /// committed-band strategies; orphans default to core.
    pub fn effective_class(&self, subdomain: &str) -> Class {
        let committed = self.committed_edges(subdomain);
        if committed.is_empty() {
            return Class::Core;
        }
        committed
            .iter()
            .filter_map(|e| e.class)
            .max()
            .unwrap_or(Class::Core)
    }

    /// Subdomains with no edge to a committed-band strategy.
    pub fn orphan_subdomains(&self) -> Vec<&str> {
        let mut v: Vec<&str> = self
            .induced
            .keys()
            .filter(|s| self.committed_edges(s).is_empty())
            .map(|s| s.as_str())
            .collect();
        v.sort();
        v
    }

    fn edge_sustains(&self, edge: &FundEdge) -> bool {
        if !edge.current {
            return false;
        }
        match crate::refs::classify(&edge.target) {
            crate::refs::RefKind::SelfFunding => true,
            crate::refs::RefKind::External(_) => true,
            crate::refs::RefKind::Path { path, .. } => self.band(&path) == Some(Band::Committed),
            _ => false,
        }
    }

    /// Committed strategies with no sustaining `current` edge (item 20).
    pub fn economic_orphans(&self) -> Vec<&str> {
        self.committed()
            .into_iter()
            .filter(|s| {
                !self
                    .funded
                    .get(*s)
                    .map(|edges| edges.iter().any(|e| self.edge_sustains(e)))
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Item 21: at least one committed strategy captures its own revenue
    /// (`current` `self` edge) or is sustained from outside the root (an
    /// external-ref edge).
    pub fn capture_point_exists(&self) -> bool {
        self.committed().into_iter().any(|s| {
            self.funded
                .get(s)
                .map(|edges| {
                    edges.iter().any(|e| {
                        e.current
                            && matches!(
                                crate::refs::classify(&e.target),
                                crate::refs::RefKind::SelfFunding
                                    | crate::refs::RefKind::External(_)
                            )
                    })
                })
                .unwrap_or(false)
        })
    }

    /// A `ready` plan whose `awaits:` targets are not all `retired` is held
    /// (skipped, still ready). Returns the first holding target and its
    /// status (`None` = target missing).
    pub fn hold(&self, plan: &str) -> Option<(String, Option<PlanStatus>)> {
        let plan = crate::tree::live_path(plan);
        if self.plan_status.get(plan).copied().flatten() != Some(PlanStatus::Ready) {
            return None;
        }
        for target in self.plan_awaits.get(plan)? {
            match self.plan_status.get(target) {
                None => return Some((target.clone(), None)),
                Some(status) if *status != Some(PlanStatus::Retired) => {
                    return Some((target.clone(), *status));
                }
                _ => {}
            }
        }
        None
    }

    /// Cycles in the `awaits:` graph (item 22). Each cycle is a path of plan
    /// rels; edges only exist between plans present in the tree.
    pub fn awaits_cycles(&self) -> Vec<Vec<String>> {
        cycles_in(&self.plan_awaits)
    }

    /// Cycles in the decision `supersedes:` graph (item 26) — a cycle erases
    /// liveness for every member, so it is a violation, not a curiosity.
    pub fn decision_supersession_cycles(&self) -> Vec<Vec<String>> {
        cycles_in(&self.decision_supersedes)
    }

    /// Target decision → the accepted successor that supersedes it. A
    /// decision is live iff it has no entry here (spec rule 6). When several
    /// successors name the same target, the lexicographically first labels it.
    pub fn superseded_decisions(&self) -> HashMap<&str, &str> {
        let mut out: HashMap<&str, &str> = HashMap::new();
        let mut successors: Vec<&String> = self.decision_supersedes.keys().collect();
        successors.sort();
        for successor in successors {
            for target in &self.decision_supersedes[successor] {
                out.entry(target.as_str()).or_insert(successor.as_str());
            }
        }
        out
    }

    /// A plan's effective class: strictest across its subdomains' effective
    /// classes; `None` when it declares no resolvable subdomain.
    pub fn plan_class(&self, tree: &Tree, plan: &str) -> Option<Class> {
        let artifact = tree.get_addressed(plan)?;
        let subs = artifact.fm.as_ref()?.get_list("subdomains")?;
        subs.iter()
            .filter(|s| self.induced.contains_key(s.as_str()))
            .map(|s| self.effective_class(s))
            .max()
    }
}

/// Depth-first cycle collection over an adjacency map. Edges only exist
/// between nodes present as keys — a node with no outgoing edges cannot sit
/// on a cycle, so the filter drops nothing a cycle needs.
fn cycles_in(graph: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
    let mut cycles: Vec<Vec<String>> = Vec::new();
    let mut done: HashSet<String> = HashSet::new();
    let mut nodes: Vec<&String> = graph.keys().collect();
    nodes.sort();

    for start in nodes {
        if done.contains(start.as_str()) {
            continue;
        }
        let mut stack: Vec<(String, usize)> = vec![(start.clone(), 0)];
        let mut path: Vec<String> = vec![];
        let mut on_path: HashSet<String> = HashSet::new();

        while let Some((node, edge_idx)) = stack.pop() {
            if edge_idx == 0 {
                path.push(node.clone());
                on_path.insert(node.clone());
            }
            let targets = graph
                .get(&node)
                .map(|v| {
                    v.iter()
                        .filter(|t| graph.contains_key(*t))
                        .cloned()
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            if edge_idx < targets.len() {
                let target = targets[edge_idx].clone();
                stack.push((node.clone(), edge_idx + 1));
                if on_path.contains(&target) {
                    let pos = path.iter().position(|p| p == &target).unwrap();
                    let mut cycle = path[pos..].to_vec();
                    cycle.sort();
                    if !cycles.contains(&cycle) {
                        cycles.push(cycle);
                    }
                } else if !done.contains(&target) {
                    stack.push((target, 0));
                }
            } else {
                done.insert(node.clone());
                on_path.remove(&node);
                path.pop();
            }
        }
    }
    cycles
}

/// How each accepted decision is reached from the operative record (item 26,
/// Decision hatching pattern). `registry` is `trellis.toml`'s decision
/// registry — passed in because instance registries are not the graph's to
/// read. Sorted by rel; drafts stay out (an unaccepted decision is still
/// being authored).
#[derive(Debug)]
pub struct DecisionReach {
    pub rel: String,
    /// The accepted successor whose `supersedes:` names this decision.
    pub superseded_by: Option<String>,
    /// Live authored artifacts outside `decisions/` citing it — a path ref,
    /// a `decision NNNN` mention, or a plan's `decisions:` entry (globs
    /// count every decision they match).
    pub cited_by: Vec<String>,
    /// Declares `Standing guidance: none` in its own body.
    pub inert: bool,
    /// Listed in the decision registry.
    pub registered: bool,
}

impl DecisionReach {
    /// Live but reachable from nothing — the item-26 judgment queue.
    pub fn unreachable(&self) -> bool {
        self.superseded_by.is_none() && self.cited_by.is_empty() && !self.inert && !self.registered
    }
}

pub fn decision_reach(tree: &Tree, derived: &Derived, registry: &[String]) -> Vec<DecisionReach> {
    let path_re = regex::Regex::new(r"decisions/\d{4}-[A-Za-z0-9_-]+\.md").unwrap();
    let mention_re = regex::Regex::new(r"(?i)\bdecision\s+(\d{4})\b").unwrap();
    let inert_re = regex::Regex::new(r"(?i)standing guidance:\s*none").unwrap();

    let decisions: Vec<&crate::tree::Artifact> = {
        let mut v: Vec<_> = tree
            .by_kind(Kind::Decision)
            .filter(|a| a.status().as_deref() == Some("accepted"))
            .collect();
        v.sort_by(|a, b| a.rel.cmp(&b.rel));
        v
    };

    let mut cited: HashMap<String, Vec<String>> = HashMap::new();
    for a in &tree.artifacts {
        if a.kind == Kind::Decision || a.archived || a.provenance().as_deref() != Some("authored") {
            continue;
        }
        let mut hits: HashSet<String> = HashSet::new();
        for m in path_re.find_iter(&a.text) {
            hits.insert(m.as_str().to_string());
        }
        for c in mention_re.captures_iter(&a.text) {
            let prefix = format!("decisions/{}-", &c[1]);
            for d in &decisions {
                if d.rel.starts_with(&prefix) {
                    hits.insert(d.rel.clone());
                }
            }
        }
        if a.kind == Kind::Plan {
            for r in
                a.fm.as_ref()
                    .and_then(|fm| fm.get_list("decisions"))
                    .unwrap_or_default()
            {
                if let Some(prefix) = r.split_once('*').map(|(p, _)| p) {
                    for d in &decisions {
                        if d.rel.starts_with(prefix) {
                            hits.insert(d.rel.clone());
                        }
                    }
                }
            }
        }
        for hit in hits {
            cited.entry(hit).or_default().push(a.rel.clone());
        }
    }

    let superseded = derived.superseded_decisions();
    decisions
        .iter()
        .map(|d| {
            let mut cited_by = cited.get(&d.rel).cloned().unwrap_or_default();
            cited_by.sort();
            cited_by.dedup();
            DecisionReach {
                rel: d.rel.clone(),
                superseded_by: superseded.get(d.rel.as_str()).map(|s| s.to_string()),
                cited_by,
                inert: inert_re.is_match(d.body()),
                registered: registry.contains(&d.rel),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_graph(edges: &[(&str, &[&str])]) -> Derived {
        let mut d = Derived::default();
        for (plan, awaits) in edges {
            d.plan_status
                .insert(plan.to_string(), Some(PlanStatus::Ready));
            d.plan_awaits.insert(
                plan.to_string(),
                awaits.iter().map(|s| s.to_string()).collect(),
            );
        }
        d
    }

    #[test]
    fn cycle_detection() {
        let d = plan_graph(&[
            ("plans/a.md", &["plans/b.md"]),
            ("plans/b.md", &["plans/c.md"]),
            ("plans/c.md", &["plans/a.md"]),
            ("plans/d.md", &[]),
        ]);
        let cycles = d.awaits_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0], vec!["plans/a.md", "plans/b.md", "plans/c.md"]);

        let acyclic = plan_graph(&[
            ("plans/a.md", &["plans/b.md", "plans/c.md"]),
            ("plans/b.md", &["plans/c.md"]),
            ("plans/c.md", &[]),
        ]);
        assert!(acyclic.awaits_cycles().is_empty());
    }

    #[test]
    fn self_cycle() {
        let d = plan_graph(&[("plans/a.md", &["plans/a.md"])]);
        assert_eq!(d.awaits_cycles(), vec![vec!["plans/a.md".to_string()]]);
    }
}
