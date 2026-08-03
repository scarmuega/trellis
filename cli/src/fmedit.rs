//! Format-preserving frontmatter edits. Never re-serializes YAML: a scalar
//! set is a single-line rewrite inside the frontmatter block, a list append
//! splices the existing form, and everything else is a refusal. Every edit
//! is verified by re-parse before the atomic rename — clever line surgery
//! must fail loudly, never corrupt silently.

use std::path::Path;

use anyhow::{bail, Context};

use crate::frontmatter;

/// Detect the file's dominant line ending.
fn eol(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

struct Doc {
    /// All lines, without endings.
    lines: Vec<String>,
    eol: &'static str,
    trailing_newline: bool,
    /// 0-based indexes of the opening and closing `---` lines.
    open: usize,
    close: usize,
}

impl Doc {
    fn load(path: &Path) -> anyhow::Result<Doc> {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read {}", path.display()))?;
        let eol = eol(&text);
        let trailing_newline = text.ends_with('\n');
        let lines: Vec<String> = text.lines().map(str::to_string).collect();
        if lines.first().map(|l| l.trim_end() != "---").unwrap_or(true) {
            bail!("{} has no frontmatter block", path.display());
        }
        let close = lines
            .iter()
            .enumerate()
            .skip(1)
            .find(|(_, l)| l.trim_end() == "---")
            .map(|(i, _)| i)
            .ok_or_else(|| {
                anyhow::anyhow!("{} has an unclosed frontmatter block", path.display())
            })?;
        Ok(Doc {
            lines,
            eol,
            trailing_newline,
            open: 0,
            close,
        })
    }

    /// 0-based line indexes of top-level `key:` lines in the block.
    fn key_lines(&self, key: &str) -> Vec<usize> {
        (self.open + 1..self.close)
            .filter(|&i| {
                self.lines[i]
                    .strip_prefix(key)
                    .map(|r| r.starts_with(':'))
                    .unwrap_or(false)
            })
            .collect()
    }

    fn render(&self) -> String {
        let mut out = self.lines.join(self.eol);
        if self.trailing_newline {
            out.push_str(self.eol);
        }
        out
    }

    fn guard_shapes(&self, key_line: usize) -> anyhow::Result<()> {
        let line = &self.lines[key_line];
        let value = line.split_once(':').map(|x| x.1).unwrap_or("").trim();
        if value.starts_with('|') || value.starts_with('>') {
            bail!("refusing to edit a block scalar (| or >) — hand-edit this field");
        }
        if value.starts_with('&') || value.starts_with('*') {
            bail!("refusing to edit near YAML anchors/aliases — hand-edit this field");
        }
        Ok(())
    }
}

fn write_atomic(path: &Path, content: &str) -> anyhow::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = tempfile::NamedTempFile::new_in(dir)?;
    std::fs::write(tmp.path(), content)?;
    tmp.persist(path)
        .map_err(|e| anyhow::anyhow!("atomic rename failed: {e}"))?;
    Ok(())
}

/// Read a scalar field.
pub fn get(path: &Path, key: &str) -> anyhow::Result<Option<String>> {
    let text = std::fs::read_to_string(path)?;
    let fm = frontmatter::extract(&text)
        .ok_or_else(|| anyhow::anyhow!("{} has no frontmatter block", path.display()))?;
    Ok(fm.get_str(key))
}

/// Set (or with `create`, insert) a top-level scalar field, preserving
/// everything else byte-for-byte, inline comment included.
pub fn set_scalar(path: &Path, key: &str, value: &str, create: bool) -> anyhow::Result<()> {
    if value.contains('\n') {
        bail!("scalar values cannot contain newlines");
    }
    let mut doc = Doc::load(path)?;
    let hits = doc.key_lines(key);
    match hits.len() {
        0 => {
            if !create {
                bail!(
                    "{} has no top-level {key}: — pass --create to add it",
                    path.display()
                );
            }
            doc.lines.insert(doc.close, format!("{key}: {value}"));
        }
        1 => {
            let i = hits[0];
            doc.guard_shapes(i)?;
            // A key whose value is a nested block (list/map) cannot take a
            // scalar without dropping lines — refuse.
            let bare = doc.lines[i]
                .split_once(':')
                .map(|x| x.1)
                .unwrap_or("")
                .trim();
            let has_children = doc
                .lines
                .get(i + 1)
                .map(|next| {
                    i + 1 < doc.close
                        && (next.starts_with(' ') || next.starts_with('\t'))
                        && !next.trim().is_empty()
                })
                .unwrap_or(false);
            if bare.is_empty() && has_children {
                bail!("{key}: holds a nested block — refusing a scalar overwrite");
            }
            let comment = comment_suffix(&doc.lines[i]);
            doc.lines[i] = format!("{key}: {value}{comment}");
        }
        n => bail!(
            "{} declares {key}: {n} times — fix the duplicate first",
            path.display()
        ),
    }

    let rendered = doc.render();
    verify(&rendered, path, |fm| {
        if fm.get_str(key).as_deref() != Some(value) {
            bail!("post-edit verification failed: {key} reads back as {:?}, expected {value:?} (value collides with YAML syntax?)", fm.get_str(key));
        }
        Ok(())
    })?;
    write_atomic(path, &rendered)
}

/// Append an entry to a top-level list field, in whichever form it already
/// uses (flow `[a, b]` or block `- item`); creates `key: [value]` if absent.
pub fn append_list(path: &Path, key: &str, value: &str) -> anyhow::Result<()> {
    if value.contains('\n') || value.contains(',') || value.contains(']') {
        bail!("list entries cannot contain newlines, commas, or ]");
    }
    let mut doc = Doc::load(path)?;
    let hits = doc.key_lines(key);
    match hits.len() {
        0 => {
            doc.lines.insert(doc.close, format!("{key}: [{value}]"));
        }
        1 => {
            let i = hits[0];
            doc.guard_shapes(i)?;
            let line = doc.lines[i].clone();
            let after_colon = line
                .split_once(':')
                .map(|x| x.1)
                .unwrap_or("")
                .trim()
                .to_string();
            if let Some(bracket) = line.rfind(']') {
                // Flow form.
                let empty = after_colon == "[]";
                let sep = if empty { "" } else { ", " };
                let mut new_line = line.clone();
                new_line.insert_str(bracket, &format!("{sep}{value}"));
                doc.lines[i] = new_line;
            } else if after_colon.is_empty() {
                // Block form: find the last `- ` item directly under the key.
                let mut insert_at = i + 1;
                let mut indent = "  ".to_string();
                let mut j = i + 1;
                while j < doc.close {
                    let l = &doc.lines[j];
                    let t = l.trim_start();
                    if t.starts_with('-') && l.starts_with(' ') {
                        indent = l[..l.len() - t.len()].to_string();
                        insert_at = j + 1;
                        j += 1;
                    } else if t.is_empty() {
                        j += 1;
                    } else {
                        break;
                    }
                }
                doc.lines.insert(insert_at, format!("{indent}- {value}"));
            } else {
                bail!("{key}: holds a scalar, not a list — refusing to append");
            }
        }
        n => bail!(
            "{} declares {key}: {n} times — fix the duplicate first",
            path.display()
        ),
    }

    let rendered = doc.render();
    verify(&rendered, path, |fm| {
        let list = fm.get_list(key).unwrap_or_default();
        if !list.iter().any(|v| v == value) {
            bail!(
                "post-edit verification failed: {key} does not contain {value:?} after the append"
            );
        }
        Ok(())
    })?;
    write_atomic(path, &rendered)
}

fn comment_suffix(line: &str) -> String {
    // Preserve an inline `  # comment` (never part of the value we write).
    match line.find(" #") {
        Some(pos) => line[pos..].to_string(),
        None => String::new(),
    }
}

fn verify(
    rendered: &str,
    path: &Path,
    intent: impl Fn(&frontmatter::Frontmatter) -> anyhow::Result<()>,
) -> anyhow::Result<()> {
    let fm = frontmatter::extract(rendered).ok_or_else(|| {
        anyhow::anyhow!(
            "post-edit verification failed: {} would lose its frontmatter block",
            path.display()
        )
    })?;
    if !fm.parsed() {
        bail!(
            "post-edit verification failed: {} frontmatter would no longer parse as YAML",
            path.display()
        );
    }
    intent(&fm)
}

/// Append body content at the end of a named section (creating the heading
/// at EOF when absent). Used for escalation records.
pub fn append_to_section(path: &Path, heading: &str, block: &str) -> anyhow::Result<()> {
    let text =
        std::fs::read_to_string(path).with_context(|| format!("cannot read {}", path.display()))?;
    let ending = eol(&text);
    let fm_close = frontmatter::extract(&text)
        .map(|f| f.close_line)
        .unwrap_or(0);
    let (headings, _) = crate::markdown::scan(&text, fm_close);
    let want = crate::markdown::slugify(heading);

    let lines: Vec<&str> = text.lines().collect();
    let mut out: Vec<String> = lines.iter().map(|s| s.to_string()).collect();

    match headings.iter().position(|h| h.slug == want) {
        None => {
            if !out.last().map(|l| l.is_empty()).unwrap_or(true) {
                out.push(String::new());
            }
            out.push(format!("## {heading}"));
            out.push(String::new());
            out.extend(block.lines().map(str::to_string));
        }
        Some(pos) => {
            let section_end = headings
                .iter()
                .skip(pos + 1)
                .find(|h| h.level <= headings[pos].level)
                .map(|h| (h.line - 1) as usize)
                .unwrap_or(out.len());
            let mut insert_at = section_end;
            while insert_at > 0
                && out
                    .get(insert_at - 1)
                    .map(|l| l.trim().is_empty())
                    .unwrap_or(false)
            {
                insert_at -= 1;
            }
            let mut chunk: Vec<String> = vec![String::new()];
            chunk.extend(block.lines().map(str::to_string));
            out.splice(insert_at..insert_at, chunk);
        }
    }

    let mut rendered = out.join(ending);
    rendered.push_str(ending);
    write_atomic(path, &rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn temp_with(content: &str) -> tempfile::NamedTempFile {
        let mut f = tempfile::Builder::new().suffix(".md").tempfile().unwrap();
        f.write_all(content.as_bytes()).unwrap();
        f
    }

    const PLAN: &str = "---\nprovenance: authored\nowner: org/founder\nstatus: ready # released 2026-07-01\ntype: initiative\nawaits: [plans/a.md]\nsubdomains:\n  - problem/x.md\n---\n# Body\n";

    #[test]
    fn scalar_set_preserves_everything_else() {
        let f = temp_with(PLAN);
        set_scalar(f.path(), "status", "active", false).unwrap();
        let text = std::fs::read_to_string(f.path()).unwrap();
        assert!(text.contains("status: active # released 2026-07-01"));
        assert_eq!(
            text.replace("status: active", "status: ready"),
            PLAN,
            "only the status line may change"
        );
    }

    #[test]
    fn scalar_set_refuses_missing_without_create() {
        let f = temp_with(PLAN);
        assert!(set_scalar(f.path(), "complexity", "deep", false).is_err());
        set_scalar(f.path(), "complexity", "deep", true).unwrap();
        assert_eq!(
            get(f.path(), "complexity").unwrap().as_deref(),
            Some("deep")
        );
    }

    #[test]
    fn scalar_set_refuses_nested_block() {
        let f = temp_with(PLAN);
        assert!(set_scalar(f.path(), "subdomains", "problem/y.md", false).is_err());
    }

    #[test]
    fn list_append_flow_and_block() {
        let f = temp_with(PLAN);
        append_list(f.path(), "awaits", "plans/b.md").unwrap();
        append_list(f.path(), "subdomains", "problem/y.md").unwrap();
        let text = std::fs::read_to_string(f.path()).unwrap();
        assert!(text.contains("awaits: [plans/a.md, plans/b.md]"));
        assert!(text.contains("  - problem/x.md\n  - problem/y.md"));
    }

    #[test]
    fn verification_rejects_yaml_breaking_value() {
        let f = temp_with(PLAN);
        let err = set_scalar(f.path(), "status", "x # y", false);
        assert!(
            err.is_err(),
            "a value the parser reads back differently must refuse"
        );
        // File untouched on refusal.
        assert_eq!(std::fs::read_to_string(f.path()).unwrap(), PLAN);
    }

    #[test]
    fn section_append_creates_and_extends() {
        let f = temp_with(PLAN);
        append_to_section(f.path(), "Escalations", "```yaml\nstatus: open\n```").unwrap();
        append_to_section(f.path(), "Escalations", "```yaml\nstatus: resolved\n```").unwrap();
        let text = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(text.matches("## Escalations").count(), 1);
        assert_eq!(text.matches("```yaml").count(), 2);
    }
}
