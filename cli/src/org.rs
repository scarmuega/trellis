//! Role plumbing shared across surfaces: how a role's holder resolves. One
//! resolver for what `/api/org`, the orgchart view, and the dispatch scan
//! each answered inline — ref.md, then system.md, then the mandate's
//! `holder:` field (decision 0045).

use crate::tree::Tree;

/// What sits behind `org/{role}/holder/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HolderKind {
    /// `holder/ref.md` — a pointer to an external identity (agent or human).
    Ref,
    /// `holder/system.md` — a local agent package.
    Package,
    /// Neither file; the mandate's `holder:` field is all there is.
    None,
}

#[derive(Debug, Clone)]
pub struct Holder {
    pub kind: HolderKind,
    /// `ref.md`'s `ref:` value, when `kind` is `Ref`.
    pub reference: Option<String>,
    /// `ref.md`'s optional `kind:` discriminant — `agent` or `human`. The
    /// split was prose-only before 0045; absence means undeclared, and every
    /// consumer must treat undeclared as the pre-0045 behavior.
    pub ref_kind: Option<String>,
}

impl Holder {
    /// Declared human — the one state that short-circuits dispatch into a
    /// handoff. Undeclared is not human: a missing `kind:` keeps the
    /// pre-discriminant behavior (spawn and let the session decide).
    pub fn is_declared_human(&self) -> bool {
        self.ref_kind.as_deref() == Some("human")
    }
}

/// The role's mandate, rendered for a spawn prompt (decision 0058): heading,
/// framing, then the file verbatim — frontmatter included, because the
/// declared fields *are* the mandate. Empty when the mandate is missing, so
/// a template may name `{mandate}` unconditionally. Accepts `coder` or
/// `org/coder`.
pub fn mandate_block(tree: &Tree, role: &str) -> String {
    let short = role.strip_prefix("org/").unwrap_or(role);
    let rel = format!("org/{short}/mandate.md");
    match tree.get(&rel) {
        Some(a) => format!(
            "## Mandate — {rel}\n\n\
             Your mandate, rendered here at spawn so the procedure's read is \
             already done; the file itself is authoritative if they have \
             diverged.\n\n{}",
            a.text.trim()
        ),
        None => String::new(),
    }
}

/// The role's local holder package, rendered for a spawn prompt (decision
/// 0058). Only a `Package` holder renders: a ref naming a plugin agent still
/// adopts inline per the procedure, a declared human never reaches a
/// runtime-triggered spawn, and rendering nothing keeps the procedure's
/// holder branch authoritative for both. Accepts `coder` or `org/coder`.
pub fn holder_block(tree: &Tree, role: &str) -> String {
    let short = role.strip_prefix("org/").unwrap_or(role);
    if holder(tree, short).kind != HolderKind::Package {
        return String::new();
    }
    let rel = format!("org/{short}/holder/system.md");
    match tree.get(&rel) {
        Some(a) => format!(
            "## Holder — {rel}\n\n\
             Your holder package's operating instructions, rendered here at \
             spawn — the procedure's adopt step is already done for the \
             local-package case.\n\n{}",
            a.text.trim()
        ),
        None => String::new(),
    }
}

/// Resolve a role's holder. `role_short` is the bare name (`coder`, not
/// `org/coder`).
pub fn holder(tree: &Tree, role_short: &str) -> Holder {
    let ref_fm = tree
        .get(&format!("org/{role_short}/holder/ref.md"))
        .and_then(|a| a.fm.as_ref());
    if let Some(fm) = ref_fm {
        return Holder {
            kind: HolderKind::Ref,
            reference: fm.get_str("ref"),
            ref_kind: fm.get_str("kind"),
        };
    }
    if tree
        .get(&format!("org/{role_short}/holder/system.md"))
        .is_some()
    {
        return Holder {
            kind: HolderKind::Package,
            reference: None,
            ref_kind: None,
        };
    }
    Holder {
        kind: HolderKind::None,
        reference: None,
        ref_kind: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::root::Root;

    fn root() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(
            dir.path().join("trellis.toml"),
            format!("spec = {}\n", crate::spec_version()),
        )
        .unwrap();
        dir
    }

    fn write(dir: &tempfile::TempDir, rel: &str, text: &str) {
        let path = dir.path().join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    fn load(dir: &tempfile::TempDir) -> Tree {
        Tree::load(Root {
            path: dir.path().to_path_buf(),
        })
        .unwrap()
    }

    #[test]
    fn a_mandate_renders_whole_and_a_missing_one_renders_nothing() {
        let dir = root();
        write(
            &dir,
            "org/coder/mandate.md",
            "---\nprovenance: authored\nescalate-to: org/founder\n---\n# Coder\n",
        );
        let tree = load(&dir);
        let block = mandate_block(&tree, "org/coder");
        assert!(block.starts_with("## Mandate — org/coder/mandate.md"));
        assert!(block.contains("escalate-to: org/founder"), "{block}");
        assert!(block.contains("already done"));
        assert_eq!(mandate_block(&tree, "ghost"), "");
    }

    #[test]
    fn only_a_package_holder_renders() {
        let dir = root();
        write(&dir, "org/coder/holder/system.md", "# Operating notes\n");
        write(
            &dir,
            "org/qa/holder/ref.md",
            "---\nref: trellis:steward\nkind: agent\n---\n",
        );
        let tree = load(&dir);
        let block = holder_block(&tree, "coder");
        assert!(block.starts_with("## Holder — org/coder/holder/system.md"));
        assert!(block.contains("Operating notes"), "{block}");
        // A ref holder adopts inline per the procedure; a missing holder has
        // nothing to say. Both render empty.
        assert_eq!(holder_block(&tree, "qa"), "");
        assert_eq!(holder_block(&tree, "ghost"), "");
    }
}
