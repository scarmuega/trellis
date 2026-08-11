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
