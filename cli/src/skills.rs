//! The spawn prompt's skill index (decision 0055): resolve the skills a
//! session is entitled to — the acting role's holder package plus, for a
//! plan-carrying session, each bounded context the plan names — and render
//! a self-contained markdown index. Names and paths, never bodies: the
//! session reads a skill's file lazily through its own tools, so the
//! mechanism asks nothing of the harness beyond a prompt and file reads.
//!
//! Skills have exactly two homes (decision 0021): `solution/{bc}/skills/`
//! for business procedure and `org/{role}/holder/skills/` for identity-bound
//! technique. A skill is a directory; its name is the directory name (spec
//! rule 10 — names are retrieval keys). Domain skills declare no
//! name/description frontmatter, so the description is derived leniently,
//! the way every frontmatter read here is: `description:` if present, else
//! the first H1, else the first prose line, else nothing.

use crate::tree::{Artifact, Tree};

/// One index entry: a skill directory resolved to a line.
pub struct Skill {
    /// The directory name — the retrieval key.
    pub name: String,
    /// The primary doc's rel, or `{dir}/` when the directory has none — a
    /// body-less skill still gets its line, never a silent drop.
    pub path: String,
    pub description: Option<String>,
}

/// Skills directly under `{base}/skills/`, in walk order. Ordering is
/// deterministic because `tree.dirs` inherits the walker's
/// `sort_by_file_name`; a parallel walk would silently break this.
pub fn under(tree: &Tree, base: &str) -> Vec<Skill> {
    tree.subdirs(&format!("{base}/skills"))
        .into_iter()
        .map(|dir| {
            let name = dir.rsplit('/').next().unwrap_or(dir).to_string();
            match primary_doc(tree, dir) {
                Some(doc) => Skill {
                    name,
                    path: doc.rel.clone(),
                    description: describe(doc),
                },
                None => Skill {
                    name,
                    path: format!("{dir}/"),
                    description: None,
                },
            }
        })
        .collect()
}

/// The `{skills}` value for a plan-carrying session (act and operator
/// errands): the owner's holder skills first — technique travels with the
/// identity — then each `contexts:` home's, in the plan's declared order,
/// duplicates collapsed first-wins.
pub fn for_plan(tree: &Tree, plan_rel: &str, owner: &str) -> String {
    let short = owner.strip_prefix("org/").unwrap_or(owner);
    let mut skills = under(tree, &format!("org/{short}/holder"));
    let contexts = tree
        .get_addressed(plan_rel)
        .and_then(|p| p.fm.as_ref())
        .and_then(|fm| fm.get_list("contexts"))
        .unwrap_or_default();
    let mut seen: Vec<&str> = Vec::new();
    for ctx in &contexts {
        let ctx = ctx.trim_end_matches('/');
        if seen.contains(&ctx) {
            continue;
        }
        seen.push(ctx);
        skills.extend(under(tree, ctx));
    }
    render(&skills)
}

/// The `{skills}` value for a ritual: the executor's holder skills only —
/// rituals carry no plan, so no context home applies.
pub fn for_ritual(tree: &Tree, executor: &str) -> String {
    let short = executor.strip_prefix("org/").unwrap_or(executor);
    render(&under(tree, &format!("org/{short}/holder")))
}

/// The self-contained block — heading and standing instruction included —
/// or the empty string, so a template may name `{skills}` unconditionally.
pub fn render(skills: &[Skill]) -> String {
    if skills.is_empty() {
        return String::new();
    }
    let mut out = String::from(
        "## Skills\n\nBefore doing work a skill below covers, read its file in full; \
         the skill is authoritative over your general approach where they overlap.\n",
    );
    for s in skills {
        out.push_str(&match &s.description {
            Some(d) => format!("\n- {} — {} → {}", s.name, d, s.path),
            None => format!("\n- {} → {}", s.name, s.path),
        });
    }
    out
}

/// The directory's primary doc: `README.md`, then `SKILL.md` (the spelling
/// plugin-style skill repos import), then the sole direct-child `.md` — a
/// `references/` subtree never competes, and two siblings are no answer.
fn primary_doc<'a>(tree: &'a Tree, dir: &str) -> Option<&'a Artifact> {
    if let Some(doc) = tree.get(&format!("{dir}/README.md")) {
        return Some(doc);
    }
    if let Some(doc) = tree.get(&format!("{dir}/SKILL.md")) {
        return Some(doc);
    }
    let prefix = format!("{dir}/");
    let mut sole = None;
    for a in &tree.artifacts {
        if let Some(rest) = a.rel.strip_prefix(&prefix) {
            if !rest.contains('/') && rest.ends_with(".md") {
                if sole.is_some() {
                    return None;
                }
                sole = Some(a);
            }
        }
    }
    sole
}

/// One line of description, derived: frontmatter `description:` (first line
/// only — a block scalar may carry a paragraph, the index carries a line),
/// else the first H1's text, else the first prose line of the body.
fn describe(doc: &Artifact) -> Option<String> {
    if let Some(d) = doc.fm.as_ref().and_then(|fm| fm.get_str("description")) {
        let first = d.lines().next().unwrap_or("").trim();
        if !first.is_empty() {
            return Some(first.to_string());
        }
    }
    if let Some(h) = doc.headings.iter().find(|h| h.level == 1) {
        return Some(h.text.clone());
    }
    doc.body()
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
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
    fn a_readme_outranks_skill_md_which_outranks_a_sole_doc() {
        let dir = root();
        write(&dir, "solution/shop/skills/a/README.md", "# A\n");
        write(&dir, "solution/shop/skills/a/SKILL.md", "# not this\n");
        write(&dir, "solution/shop/skills/b/SKILL.md", "# B\n");
        write(&dir, "solution/shop/skills/b/notes.md", "# not this\n");
        write(&dir, "solution/shop/skills/c/notes.md", "# C\n");
        write(&dir, "solution/shop/skills/d/one.md", "# one\n");
        write(&dir, "solution/shop/skills/d/two.md", "# two\n");
        let tree = load(&dir);
        let skills = under(&tree, "solution/shop");
        let paths: Vec<&str> = skills.iter().map(|s| s.path.as_str()).collect();
        assert_eq!(
            paths,
            vec![
                "solution/shop/skills/a/README.md",
                "solution/shop/skills/b/SKILL.md",
                "solution/shop/skills/c/notes.md",
                // Two sibling docs and no canonical name is no answer: the
                // directory itself is the address.
                "solution/shop/skills/d/",
            ]
        );
    }

    #[test]
    fn description_prefers_frontmatter_then_h1_then_first_prose_line() {
        let dir = root();
        write(
            &dir,
            "solution/shop/skills/a/README.md",
            "---\ndescription: |\n  Cost a kit\n  from its bill of materials.\n---\n# Ignored\n",
        );
        write(
            &dir,
            "solution/shop/skills/b/README.md",
            "# From the H1\n\nprose\n",
        );
        write(
            &dir,
            "solution/shop/skills/c/README.md",
            "## minor\n\nJust prose.\n",
        );
        let tree = load(&dir);
        let skills = under(&tree, "solution/shop");
        let descs: Vec<Option<&str>> = skills.iter().map(|s| s.description.as_deref()).collect();
        assert_eq!(
            descs,
            vec![
                // A block scalar keeps only its first line — one line per skill.
                Some("Cost a kit"),
                Some("From the H1"),
                Some("Just prose."),
            ]
        );
    }

    #[test]
    fn a_docless_skill_still_lists_by_its_directory_path() {
        let dir = root();
        std::fs::create_dir_all(dir.path().join("org/coder/holder/skills/bare")).unwrap();
        let tree = load(&dir);
        let skills = under(&tree, "org/coder/holder");
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "bare");
        assert_eq!(skills[0].path, "org/coder/holder/skills/bare/");
        assert!(skills[0].description.is_none());
    }

    #[test]
    fn owner_skills_precede_context_skills_and_duplicate_contexts_collapse() {
        let dir = root();
        write(&dir, "org/coder/holder/skills/technique/README.md", "# T\n");
        write(&dir, "solution/shop/skills/procedure/README.md", "# P\n");
        write(
            &dir,
            "plans/x.md",
            "---\nowner: org/coder\nstatus: ready\ncontexts: [solution/shop, solution/shop/]\n---\n# x\n",
        );
        let tree = load(&dir);
        let got = for_plan(&tree, "plans/x.md", "org/coder");
        let technique = got.find("technique").unwrap();
        let procedure = got.find("procedure").unwrap();
        assert!(
            technique < procedure,
            "holder technique lists first:\n{got}"
        );
        assert_eq!(
            got.matches("\n- procedure").count(),
            1,
            "duplicate context collapsed:\n{got}"
        );
    }

    #[test]
    fn no_skills_renders_the_empty_string() {
        let dir = root();
        write(
            &dir,
            "plans/x.md",
            "---\nowner: org/coder\nstatus: ready\n---\n# x\n",
        );
        let tree = load(&dir);
        assert_eq!(for_plan(&tree, "plans/x.md", "org/coder"), "");
        assert_eq!(for_ritual(&tree, "org/steward"), "");
    }

    #[test]
    fn the_rendered_block_carries_its_own_heading_and_instruction() {
        let skill = Skill {
            name: "kit-costing".into(),
            path: "solution/shop/skills/kit-costing/README.md".into(),
            description: Some("Cost a kit".into()),
        };
        let got = render(&[skill]);
        assert!(got.starts_with("## Skills\n"), "{got}");
        assert!(got.contains("read its file in full"), "{got}");
        assert!(
            got.contains("- kit-costing — Cost a kit → solution/shop/skills/kit-costing/README.md"),
            "{got}"
        );
    }
}
