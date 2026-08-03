//! Load a Trellis root into memory: every `.md` artifact parsed once
//! (frontmatter, headings, fences), plus the directory census the structural
//! lints need. Dot-directories are runtime/VCS state, never artifacts.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::frontmatter::{self, Frontmatter};
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
    Conventions,
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

pub struct Tree {
    pub root: Root,
    pub artifacts: Vec<Artifact>,
    /// Every non-dot directory, root-relative.
    pub dirs: Vec<String>,
    /// Non-markdown files (for the secrets sweep), root-relative.
    pub other_files: Vec<String>,
    index: HashMap<String, usize>,
}

pub fn classify(rel: &str) -> Kind {
    let parts: Vec<&str> = rel.split('/').collect();
    match parts.as_slice() {
        ["conventions.md"] => Kind::Conventions,
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
        let mut artifacts = Vec::new();
        let mut dirs = Vec::new();
        let mut other_files = Vec::new();

        let walker = walkdir::WalkDir::new(&root.path)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                !(e.depth() > 0
                    && e.file_name()
                        .to_str()
                        .map(|n| n.starts_with('.'))
                        .unwrap_or(false))
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
            if rel.ends_with(".md") {
                let text = std::fs::read_to_string(entry.path()).unwrap_or_default();
                let fm = frontmatter::extract(&text);
                let skip = fm.as_ref().map(|f| f.close_line).unwrap_or(0);
                let (headings, fences) = markdown::scan(&text, skip);
                artifacts.push(Artifact {
                    kind: classify(&rel),
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

        let index = artifacts
            .iter()
            .enumerate()
            .map(|(i, a)| (a.rel.clone(), i))
            .collect();
        Ok(Tree {
            root,
            artifacts,
            dirs,
            other_files,
            index,
        })
    }

    pub fn get(&self, rel: &str) -> Option<&Artifact> {
        self.index.get(rel).map(|&i| &self.artifacts[i])
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
