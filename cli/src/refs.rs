//! Reference classification and resolution. Rule 1: refs are the only
//! boundary-crossing primitive and this kernel never resolves one outside
//! the root — absolute paths and `..` escapes are refused, anything that is
//! not root-relative-path-shaped is an opaque external ref.

use crate::markdown::slugify;
use crate::tree::Tree;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefKind {
    /// `self` in a `funded-by:` edge.
    SelfFunding,
    /// `org/{role}`.
    Role(String),
    /// Root-relative path, optionally `#anchor`.
    Path {
        path: String,
        anchor: Option<String>,
    },
    /// `decisions/NNNN-*` style glob.
    Glob(String),
    /// Opaque external ref — never resolved (rule 2).
    External(String),
}

const TOP_LEVEL: &[&str] = &[
    "problem/",
    "strategy/",
    "plans/",
    "solution/",
    "metrics/",
    "decisions/",
    "org/",
];

pub fn classify(raw: &str) -> RefKind {
    let raw = raw.trim();
    if raw == "self" {
        return RefKind::SelfFunding;
    }
    if let Some(role) = raw.strip_prefix("org/") {
        if !role.is_empty() && !role.contains('/') && !role.contains('#') {
            return RefKind::Role(raw.to_string());
        }
    }
    let path_like = raw.ends_with(".md")
        || raw.contains(".md#")
        || TOP_LEVEL.iter().any(|p| raw.starts_with(p))
        || raw.starts_with('/')
        || raw.starts_with("../")
        || raw.starts_with("./");
    if !path_like {
        return RefKind::External(raw.to_string());
    }
    if raw.contains('*') {
        return RefKind::Glob(raw.to_string());
    }
    match raw.split_once('#') {
        Some((path, anchor)) => RefKind::Path {
            path: path.to_string(),
            anchor: Some(anchor.to_string()),
        },
        None => RefKind::Path {
            path: raw.to_string(),
            anchor: None,
        },
    }
}

fn escapes_root(path: &str) -> bool {
    path.starts_with('/') || path.split('/').any(|seg| seg == "..")
}

/// Resolve a ref against the tree. `Ok(())` means it resolves (or is opaque
/// external / `self`, which the kernel never checks); `Err` carries the
/// reason it does not.
pub fn resolve(raw: &str, tree: &Tree) -> Result<(), String> {
    match classify(raw) {
        RefKind::SelfFunding | RefKind::External(_) => Ok(()),
        RefKind::Role(r) => {
            if tree.role_exists(&r) {
                Ok(())
            } else {
                Err(format!(
                    "{r} is not a role in this root (no org/{{role}}/mandate.md)",
                ))
            }
        }
        RefKind::Glob(g) => {
            if escapes_root(&g) {
                return Err(format!("{g} escapes the root (rule 1)"));
            }
            let prefix = g.split('*').next().unwrap_or("");
            if tree.artifacts.iter().any(|a| a.rel.starts_with(prefix)) {
                Ok(())
            } else {
                Err(format!("nothing matches {g}"))
            }
        }
        RefKind::Path { path, anchor } => {
            if escapes_root(&path) {
                return Err(format!("{path} escapes the root (rule 1)"));
            }
            if path.ends_with(".md") {
                let Some(artifact) = tree.get_addressed(&path) else {
                    return Err(format!("{path} does not exist"));
                };
                if let Some(anchor) = anchor {
                    let want = slugify(&anchor);
                    // Slug match: heading level and case don't matter.
                    let hit = artifact.headings.iter().any(|h| h.slug == want);
                    if !hit {
                        return Err(format!("{path} has no heading matching #{anchor}"));
                    }
                }
                Ok(())
            } else {
                // Directory ref (a bounded context, a holder package).
                let path = path.trim_end_matches('/');
                if tree.has_dir_addressed(path) {
                    Ok(())
                } else {
                    Err(format!("{path} is not a directory in this root"))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification() {
        assert_eq!(classify("self"), RefKind::SelfFunding);
        assert_eq!(classify("org/steward"), RefKind::Role("org/steward".into()));
        assert!(matches!(classify("strategy/x.md"), RefKind::Path { .. }));
        assert!(matches!(
            classify("market.md#n-trail-nutrition"),
            RefKind::Path {
                anchor: Some(_),
                ..
            }
        ));
        assert!(matches!(classify("decisions/0004-*"), RefKind::Glob(_)));
        assert!(matches!(
            classify("solution/booking-desk"),
            RefKind::Path { .. }
        ));
        assert!(matches!(
            classify("Stripe billing API v3"),
            RefKind::External(_)
        ));
        assert!(matches!(
            classify("org/steward/mandate.md"),
            RefKind::Path { .. }
        ));
    }

    #[test]
    fn escapes() {
        assert!(escapes_root("/etc/passwd"));
        assert!(escapes_root("../other-root/x.md"));
        assert!(escapes_root("plans/../../x.md"));
        assert!(!escapes_root("plans/x.md"));
    }
}
