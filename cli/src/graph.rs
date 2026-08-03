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

    for a in tree.by_kind(Kind::Plan) {
        d.plan_status.insert(
            a.rel.clone(),
            a.status().and_then(|s| PlanStatus::parse(&s)),
        );
        d.plan_awaits.insert(
            a.rel.clone(),
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
        let mut cycles: Vec<Vec<String>> = Vec::new();
        let mut done: HashSet<String> = HashSet::new();
        let mut nodes: Vec<&String> = self.plan_awaits.keys().collect();
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
                let targets = self
                    .plan_awaits
                    .get(&node)
                    .map(|v| {
                        v.iter()
                            .filter(|t| self.plan_awaits.contains_key(*t))
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

    /// A plan's effective class: strictest across its subdomains' effective
    /// classes; `None` when it declares no resolvable subdomain.
    pub fn plan_class(&self, tree: &Tree, plan: &str) -> Option<Class> {
        let artifact = tree.get(plan)?;
        let subs = artifact.fm.as_ref()?.get_list("subdomains")?;
        subs.iter()
            .filter(|s| self.induced.contains_key(s.as_str()))
            .map(|s| self.effective_class(s))
            .max()
    }
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
