//! Load a Trellis root into memory: every `.md` artifact parsed once
//! (frontmatter, headings, fences), plus the directory census the structural
//! lints need. Dot-directories are runtime/VCS state, never artifacts.
//!
//! A root holds more than the domain authors — `spec/model.md` puts real code
//! under `solution/{bc}/{deployment unit}/`, and code brings markdown that
//! belongs to its own project. What is an artifact is therefore *declared*,
//! never inferred, by two statements the domain already makes (see `Scope`).

use std::collections::HashMap;
use std::path::PathBuf;

use crate::domain::DomainConfig;
use crate::frontmatter::{self, Frontmatter};
use crate::gitio::Git;
use crate::markdown::{self, FencedBlock, Heading};
use crate::root::Root;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Problem,
    Strategy,
    Plan,
    Mandate,
    HolderRef,
    HolderDoc,
    Decision,
    MetricsDefinitions,
    MetricsActual,
    Domain,
    Rituals,
    Market,
    Glossary,
    ContextMap,
    ContextDoc,
    Other,
}

#[derive(Debug)]
pub struct Artifact {
    /// Root-relative path, forward slashes.
    pub rel: String,
    pub abs: PathBuf,
    pub kind: Kind,
    /// Under the terminal tier (`archive/`). Derived from `rel`, never
    /// declared: the frontmatter's terminal status is the fact, the tier is
    /// where the fact puts the file.
    pub archived: bool,
    pub fm: Option<Frontmatter>,
    pub headings: Vec<Heading>,
    pub fences: Vec<FencedBlock>,
    pub text: String,
}

impl Artifact {
    pub fn owner(&self) -> Option<String> {
        self.fm.as_ref().and_then(|fm| fm.get_str("owner"))
    }

    pub fn provenance(&self) -> Option<String> {
        self.fm.as_ref().and_then(|fm| fm.get_str("provenance"))
    }

    pub fn status(&self) -> Option<String> {
        self.fm.as_ref().and_then(|fm| fm.get_str("status"))
    }

    /// Body text after the frontmatter block.
    pub fn body(&self) -> &str {
        match &self.fm {
            None => &self.text,
            Some(fm) => {
                let skip = fm.close_line as usize;
                let mut offset = 0;
                for (i, line) in self.text.split_inclusive('\n').enumerate() {
                    if i + 1 > skip {
                        break;
                    }
                    offset += line.len();
                }
                &self.text[offset..]
            }
        }
    }
}

/// What the walk did not take in. Two declarations and one structural fact.
///
/// The declarations: `.gitignore` says "not in this repo"; the `carried`
/// list in `trellis.toml` says "in the repo, but the domain carries it
/// rather than authors it".
///
/// The structural fact: a directory holding its own `.git` is a *different
/// repository* — a submodule (a `.git` file), a nested clone or worktree (a
/// `.git` directory). Its files belong to that repo, are not in this one's
/// index, and — per the boundary guarantees — are materialized read-only, so
/// this domain could not fix a finding in them if it wanted to. Git says so
/// itself: a submodule enters this repo's index as one gitlink, never as its
/// contents, so no ignore query will ever name it. Nothing declares this and
/// nothing needs to.
///
/// The three are enforced at two points, and the difference is the point.
/// Git-ignored paths and nested repositories are pruned from the walk
/// outright — nothing sees them, not even item 10's secrets sweep, because
/// neither is in the repo the secrets policy speaks about. A carried path is
/// still walked and still swept — it *is* tracked here — it simply stops
/// being an artifact.
#[derive(Debug, Default, Clone, serde::Serialize)]
pub struct Scope {
    /// Root-relative prefixes git excludes; empty outside a repo.
    pub git_ignored: Vec<String>,
    /// Root-relative prefixes `trellis.toml` declares carried.
    pub carried: Vec<String>,
    /// Root-relative directories holding their own `.git`. Unlike the two
    /// above this is discovered by the walk, not declared before it.
    pub nested_repos: Vec<String>,
}

impl Scope {
    pub fn discover(root: &Root, carried: &[String]) -> Scope {
        let git_ignored = Git::new(root.path.clone()).ignored_paths();

        Scope {
            git_ignored: minimal(git_ignored),
            carried: minimal(carried.to_vec()),
            nested_repos: vec![], // filled by the walk
        }
    }

    pub fn is_empty(&self) -> bool {
        self.git_ignored.is_empty() && self.carried.is_empty() && self.nested_repos.is_empty()
    }

    /// Pruned from the walk entirely.
    pub fn is_git_ignored(&self, rel: &str) -> bool {
        under_any(rel, &self.git_ignored)
    }

    /// Walked and swept, but never an artifact.
    pub fn is_carried(&self, rel: &str) -> bool {
        under_any(rel, &self.carried)
    }

    /// A submodule's `.git` is a file; a nested clone's is a directory.
    /// Either way the directory is another repository's worktree.
    fn is_nested_repo(dir: &std::path::Path) -> bool {
        dir.join(".git").exists()
    }
}

/// Whether `rel` is at or under any prefix in `prefixes`. Matching is on path
/// segments, so `node_modules` covers `node_modules/left-pad/README.md` and
/// leaves `node_modules_v2/README.md` alone. Shared by every path-narrowing
/// surface — the scope's two declarations, `lint`'s and `tree`'s positionals —
/// so a path means the same thing whichever one you hand it to.
pub fn under_any(rel: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|p| {
        let p = p.trim_end_matches('/');
        !p.is_empty() && (rel == p || rel.starts_with(&format!("{p}/")))
    })
}

/// Sorted, with any prefix already covered by a shorter one dropped. Git
/// reports both a wholly-ignored directory and its ignored child, and a
/// hand-written registry can do the same; the redundant entry changes no
/// outcome and overstates what was excluded.
fn minimal(mut prefixes: Vec<String>) -> Vec<String> {
    prefixes.sort();
    let mut out: Vec<String> = Vec::new();
    for p in prefixes {
        if !under_any(p.trim_end_matches('/'), &out) {
            out.push(p);
        }
    }
    out
}

pub struct Tree {
    pub root: Root,
    /// The root's `trellis.toml`, parsed before the walk — a tree cannot
    /// exist without it, so no consumer ever sees half a config.
    pub domain: DomainConfig,
    pub artifacts: Vec<Artifact>,
    /// Every non-dot directory, root-relative.
    pub dirs: Vec<String>,
    /// Files the domain carries but does not author: non-markdown, plus
    /// markdown under a carried-content path. Swept for secrets, never
    /// artifacts. Root-relative.
    pub other_files: Vec<String>,
    /// The declared boundary this tree was loaded under.
    pub scope: Scope,
    index: HashMap<String, usize>,
}

/// The terminal tier's prefix. An archived artifact's path is exactly its
/// live path with this in front, which is what lets one rule serve every
/// kind and what keeps the live path canonical for refs.
pub const ARCHIVE: &str = "archive/";

pub fn is_archived(rel: &str) -> bool {
    rel.starts_with(ARCHIVE)
}

/// The live path an archived artifact is addressed by — its own path
/// otherwise. Refs never name the tier (spec rule 13).
pub fn live_path(rel: &str) -> &str {
    rel.strip_prefix(ARCHIVE).unwrap_or(rel)
}

/// The status value that admits an artifact to the terminal tier, or `None`
/// for one that rides along inside an archived subtree rather than declaring
/// its own. Kinds with a lifecycle of their own use its terminal value; the
/// rest use `archived`, which is the whole of their lifecycle.
pub fn terminal_status(kind: Kind, rel: &str) -> Option<&'static str> {
    match kind {
        Kind::Plan => Some("retired"),
        Kind::Strategy => Some("discarded"),
        Kind::Problem | Kind::Mandate => Some("archived"),
        Kind::ContextDoc if live_path(rel).ends_with("/README.md") => Some("archived"),
        _ => None,
    }
}

/// Kind is read through the tier: `archive/plans/x.md` is a plan, filed
/// where finished plans go. Classification is otherwise unchanged, so the
/// mirror needs no per-kind rule.
pub fn classify(rel: &str) -> Kind {
    let rel = live_path(rel);
    let parts: Vec<&str> = rel.split('/').collect();
    match parts.as_slice() {
        ["domain.md"] => Kind::Domain,
        ["rituals.md"] => Kind::Rituals,
        ["market.md"] => Kind::Market,
        ["glossary.md"] => Kind::Glossary,
        ["problem", f] if f.ends_with(".md") && *f != "README.md" => Kind::Problem,
        ["strategy", f] if f.ends_with(".md") => Kind::Strategy,
        ["plans", f] if f.ends_with(".md") => Kind::Plan,
        ["decisions", f] if f.ends_with(".md") => Kind::Decision,
        ["metrics", "definitions.md"] => Kind::MetricsDefinitions,
        ["metrics", "actuals", ..] => Kind::MetricsActual,
        ["solution", "context-map.md"] => Kind::ContextMap,
        ["solution", _, "glossary.md"] => Kind::Glossary,
        ["solution", ..] => Kind::ContextDoc,
        ["org", _, "mandate.md"] => Kind::Mandate,
        ["org", _, "holder", "ref.md"] => Kind::HolderRef,
        ["org", _, "holder", ..] => Kind::HolderDoc,
        _ => Kind::Other,
    }
}

impl Tree {
    pub fn load(root: Root) -> anyhow::Result<Tree> {
        let domain = DomainConfig::load(&root.path)?;
        let scope = Scope::discover(&root, &domain.carried);
        Tree::load_scoped(root, domain, scope)
    }

    pub fn load_scoped(root: Root, domain: DomainConfig, mut scope: Scope) -> anyhow::Result<Tree> {
        let mut artifacts = Vec::new();
        let mut dirs = Vec::new();
        let mut other_files = Vec::new();
        // Discovered mid-walk rather than declared up front, so it is
        // collected here and folded back into the scope afterwards.
        let nested = std::cell::RefCell::new(Vec::new());

        let walker = walkdir::WalkDir::new(&root.path)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                if e.depth() == 0 {
                    return true;
                }
                if e.file_name()
                    .to_str()
                    .map(|n| n.starts_with('.'))
                    .unwrap_or(false)
                {
                    return false;
                }
                // Pruning here rather than after the fact is what keeps a
                // node_modules/ — or a submodule — from ever being enumerated.
                let Some(rel) = root.rel(e.path()) else {
                    return true;
                };
                if scope.is_git_ignored(&rel) {
                    return false;
                }
                if e.file_type().is_dir() && Scope::is_nested_repo(e.path()) {
                    nested.borrow_mut().push(rel);
                    return false;
                }
                true
            });

        for entry in walker {
            let entry = entry?;
            let Some(rel) = root.rel(entry.path()) else {
                continue;
            };
            if rel.is_empty() {
                continue;
            }
            if entry.file_type().is_dir() {
                dirs.push(rel);
                continue;
            }
            if !entry.file_type().is_file() {
                continue;
            }
            if rel.ends_with(".md") && !scope.is_carried(&rel) {
                let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
                let fm = frontmatter::extract(&text);
                let skip = fm.as_ref().map(|f| f.close_line).unwrap_or(0);
                let (headings, fences) = markdown::scan(&text, skip);
                artifacts.push(Artifact {
                    kind: classify(&rel),
                    archived: is_archived(&rel),
                    abs: entry.path().to_path_buf(),
                    rel,
                    fm,
                    headings,
                    fences,
                    text,
                });
            } else {
                other_files.push(rel);
            }
        }

        scope.nested_repos = nested.into_inner();
        scope.nested_repos.sort();

        let index = artifacts
            .iter()
            .enumerate()
            .map(|(i, a)| (a.rel.clone(), i))
            .collect();
        Ok(Tree {
            root,
            domain,
            artifacts,
            dirs,
            other_files,
            scope,
            index,
        })
    }

    pub fn get(&self, rel: &str) -> Option<&Artifact> {
        self.index.get(rel).map(|&i| &self.artifacts[i])
    }

    /// Resolve by *address* rather than by location: an artifact keeps being
    /// named by its live path after it moves into the terminal tier, so a
    /// ref written before the move still resolves (spec rule 13). Tries the
    /// path as given first, so an explicit `archive/…` still works for a
    /// human debugging — the lint owns the canonical-form complaint.
    pub fn get_addressed(&self, rel: &str) -> Option<&Artifact> {
        self.get(rel)
            .or_else(|| self.get(&format!("{ARCHIVE}{}", live_path(rel))))
    }

    /// `has_dir` through the tier, for directory refs (a bounded context, a
    /// holder package) whose subtree has been archived.
    pub fn has_dir_addressed(&self, rel: &str) -> bool {
        self.has_dir(rel) || self.has_dir(&format!("{ARCHIVE}{}", live_path(rel)))
    }

    pub fn by_kind(&self, kind: Kind) -> impl Iterator<Item = &Artifact> {
        self.artifacts.iter().filter(move |a| a.kind == kind)
    }

    pub fn has_dir(&self, rel: &str) -> bool {
        self.dirs.iter().any(|d| d == rel)
    }

    /// Direct children directories of a directory.
    pub fn subdirs(&self, rel: &str) -> Vec<&str> {
        let prefix = format!("{rel}/");
        self.dirs
            .iter()
            .filter(|d| d.starts_with(&prefix) && !d[prefix.len()..].contains('/'))
            .map(|d| d.as_str())
            .collect()
    }

    /// Role names: directories under `org/` holding a `mandate.md`.
    pub fn roles(&self) -> Vec<String> {
        self.by_kind(Kind::Mandate)
            .filter_map(|a| a.rel.split('/').nth(1).map(str::to_string))
            .collect()
    }

    /// Whether `org/{role}` resolves (has a mandate).
    pub fn role_exists(&self, role_ref: &str) -> bool {
        let Some(role) = role_ref.strip_prefix("org/") else {
            return false;
        };
        !role.contains('/') && self.get(&format!("org/{role}/mandate.md")).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scope(carried: &[&str]) -> Scope {
        Scope {
            git_ignored: vec![],
            carried: carried.iter().map(|s| s.to_string()).collect(),
            nested_repos: vec![],
        }
    }

    #[test]
    fn prefixes_match_whole_segments() {
        let s = scope(&["node_modules", "solution/app/storefront/"]);
        assert!(s.is_carried("node_modules"));
        assert!(s.is_carried("node_modules/left-pad/README.md"));
        assert!(s.is_carried("solution/app/storefront/docs/guide.md"));
        // A shared textual prefix is not a shared path.
        assert!(!s.is_carried("node_modules_v2/README.md"));
        assert!(!s.is_carried("solution/app/storefront-legacy/README.md"));
        assert!(!s.is_carried("solution/app/README.md"));
    }

    #[test]
    fn redundant_prefixes_collapse_to_the_shortest() {
        let out = minimal(vec![
            "a/node_modules/".into(),
            "a/".into(),
            "b/dist".into(),
            "a-sibling/".into(),
        ]);
        assert_eq!(out, vec!["a-sibling/", "a/", "b/dist"]);
    }

    #[test]
    fn an_empty_registry_covers_nothing() {
        let s = scope(&[]);
        assert!(!s.is_carried("problem/anything.md"));
        assert!(s.is_empty());
    }
}
