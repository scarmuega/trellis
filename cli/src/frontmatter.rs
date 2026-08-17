//! Frontmatter extraction and lenient typed access.
//!
//! The block is whatever sits between the first `---` line and the next
//! `---` line (the same boundary `gate.mjs` and the dispatch awk used).
//! Reads are lenient by design: a plan with an illegal or unparseable field
//! must still yield its other fields — full-YAML parse first, line-scan
//! fallback when the block does not parse as a mapping.

use yaml_rust2::{Yaml, YamlLoader};

#[derive(Debug)]
pub struct Frontmatter {
    /// Verbatim interior of the block (no fences).
    pub raw: String,
    /// 1-based line of the opening `---` (always 1 today).
    pub open_line: u32,
    /// 1-based line of the closing `---`.
    pub close_line: u32,
    doc: Option<Yaml>,
}

/// An `induced-by:` / `funded-by:` edge.
#[derive(Debug, Clone)]
pub struct Edge {
    pub target: String,
    pub class: Option<String>,
    pub relation: Option<String>,
}

fn fence(line: &str) -> bool {
    let t = line.trim_end();
    t == "---"
}

/// Extract the frontmatter block from full file text.
pub fn extract(text: &str) -> Option<Frontmatter> {
    let mut lines = text.split_inclusive('\n');
    let first = lines.next()?;
    if !fence(first.trim_end_matches(['\r', '\n'])) {
        return None;
    }
    let mut raw = String::new();
    for (line_no, line) in (2u32..).zip(lines) {
        let bare = line.trim_end_matches(['\r', '\n']);
        if fence(bare) {
            let doc = YamlLoader::load_from_str(&raw)
                .ok()
                .and_then(|mut docs| {
                    if docs.is_empty() {
                        None
                    } else {
                        Some(docs.remove(0))
                    }
                })
                .filter(|d| d.as_hash().is_some());
            return Some(Frontmatter {
                raw,
                open_line: 1,
                close_line: line_no,
                doc,
            });
        }
        raw.push_str(line);
    }
    None
}

fn yaml_to_string(y: &Yaml) -> Option<String> {
    match y {
        Yaml::String(s) => Some(s.clone()),
        Yaml::Integer(i) => Some(i.to_string()),
        Yaml::Real(r) => Some(r.clone()),
        Yaml::Boolean(b) => Some(b.to_string()),
        _ => None,
    }
}

impl Frontmatter {
    fn lookup(&self, key: &str) -> Option<&Yaml> {
        self.doc
            .as_ref()?
            .as_hash()?
            .get(&Yaml::String(key.to_string()))
    }

    /// Line-scan for a top-level `key:` line; returns the value part.
    fn scan_value(&self, key: &str) -> Option<String> {
        for line in self.raw.lines() {
            if let Some(rest) = line.strip_prefix(key) {
                if let Some(v) = rest.strip_prefix(':') {
                    let v = v.split(" #").next().unwrap_or("").trim();
                    let v = v.trim_matches(|c| c == '"' || c == '\'');
                    if v.is_empty() {
                        return None;
                    }
                    return Some(v.to_string());
                }
            }
        }
        None
    }

    /// A top-level key is present (regardless of value shape).
    pub fn has_key(&self, key: &str) -> bool {
        if let Some(h) = self.doc.as_ref().and_then(|d| d.as_hash()) {
            if h.contains_key(&Yaml::String(key.to_string())) {
                return true;
            }
        }
        self.raw.lines().any(|l| {
            l.strip_prefix(key)
                .map(|r| r.starts_with(':'))
                .unwrap_or(false)
        })
    }

    /// 1-based file line of a top-level key.
    pub fn line_of(&self, key: &str) -> Option<u32> {
        for (i, line) in self.raw.lines().enumerate() {
            if line
                .strip_prefix(key)
                .map(|r| r.starts_with(':'))
                .unwrap_or(false)
            {
                return Some(self.open_line + 1 + i as u32);
            }
        }
        None
    }

    /// Scalar read, lenient (parse first, line-scan fallback).
    pub fn get_str(&self, key: &str) -> Option<String> {
        match self.lookup(key) {
            Some(y) => yaml_to_string(y),
            None if self.doc.is_none() => self.scan_value(key),
            None => None,
        }
    }

    /// List read: flow `[a, b]` or block `- item` form, lenient fallback
    /// mirroring the dispatch awk when the block does not parse.
    pub fn get_list(&self, key: &str) -> Option<Vec<String>> {
        if let Some(y) = self.lookup(key) {
            return match y {
                Yaml::Array(items) => Some(items.iter().filter_map(yaml_to_string).collect()),
                Yaml::Null => Some(vec![]),
                other => yaml_to_string(other).map(|s| vec![s]),
            };
        }
        if self.doc.is_some() {
            return None;
        }
        // awk-parity scan: inline `key: [a, b]` or block list below `key:`.
        let mut out = Vec::new();
        let mut in_list = false;
        let mut seen = false;
        for line in self.raw.lines() {
            if let Some(rest) = line.strip_prefix(key) {
                if let Some(v) = rest.strip_prefix(':') {
                    seen = true;
                    let v = v.split(" #").next().unwrap_or("").trim();
                    if let Some(inner) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                        for item in inner.split(',') {
                            let item = item.trim().trim_matches(|c| c == '"' || c == '\'');
                            if !item.is_empty() {
                                out.push(item.to_string());
                            }
                        }
                        return Some(out);
                    }
                    if v.is_empty() {
                        in_list = true;
                        continue;
                    }
                    return Some(vec![v.trim_matches(|c| c == '"' || c == '\'').to_string()]);
                }
            }
            if in_list {
                let t = line.trim_start();
                if let Some(item) = t.strip_prefix("- ").or_else(|| t.strip_prefix('-')) {
                    let item = item.split(" #").next().unwrap_or("").trim();
                    if !item.is_empty() {
                        out.push(item.trim_matches(|c| c == '"' || c == '\'').to_string());
                    }
                } else if !t.is_empty() {
                    break;
                }
            }
        }
        if seen {
            Some(out)
        } else {
            None
        }
    }

    /// Edge-list read (`induced-by:`, `funded-by:`).
    pub fn get_edges(&self, key: &str, target_key: &str) -> Option<Vec<Edge>> {
        let y = self.lookup(key)?;
        let items = y.as_vec()?;
        let mut out = Vec::new();
        for item in items {
            let h = match item.as_hash() {
                Some(h) => h,
                None => {
                    // A bare string entry is a malformed edge; surface it as a
                    // target so lint can name it.
                    if let Some(s) = yaml_to_string(item) {
                        out.push(Edge {
                            target: s,
                            class: None,
                            relation: None,
                        });
                    }
                    continue;
                }
            };
            let get = |k: &str| h.get(&Yaml::String(k.to_string())).and_then(yaml_to_string);
            out.push(Edge {
                target: get(target_key).unwrap_or_default(),
                class: get("class"),
                relation: get("relation"),
            });
        }
        Some(out)
    }

    /// All top-level keys, in document order (parse path only).
    pub fn keys(&self) -> Vec<String> {
        self.doc
            .as_ref()
            .and_then(|d| d.as_hash())
            .map(|h| h.keys().filter_map(yaml_to_string).collect())
            .unwrap_or_default()
    }

    /// Whether the block parsed as a YAML mapping at all.
    pub fn parsed(&self) -> bool {
        self.doc.is_some()
    }

    /// Every declared field, shape preserved — what the author wrote, not
    /// what a consumer went looking for. Key order is the serializer's
    /// (alphabetical), not the document's: a rendering order is the view's
    /// decision, and this is the data.
    ///
    /// The typed accessors above each answer one question a caller already
    /// knew to ask, which is right for the kernel and wrong for a surface
    /// that must render a field nobody has heard of. Unparsed frontmatter
    /// yields `None` rather than a line-scanned guess: the lenient fallbacks
    /// exist so one known key still resolves when the block is malformed,
    /// and inventing a shape for every key on that path would put fiction in
    /// a view (lint item 6 owns the malformed block).
    pub fn fields(&self) -> Option<serde_json::Value> {
        let hash = self.doc.as_ref()?.as_hash()?;
        let mut out = serde_json::Map::new();
        for (k, v) in hash {
            if let Some(key) = yaml_to_string(k) {
                out.insert(key, yaml_to_json(v));
            }
        }
        Some(serde_json::Value::Object(out))
    }
}

/// A YAML node as JSON, so a field's declared shape survives the trip to a
/// consumer that cannot know the schema in advance. Dates and other
/// unquoted scalars land as the strings they were written as — the tree's
/// own vocabulary, never a reformatted one.
fn yaml_to_json(y: &Yaml) -> serde_json::Value {
    match y {
        Yaml::String(s) => s.clone().into(),
        Yaml::Integer(i) => (*i).into(),
        Yaml::Boolean(b) => (*b).into(),
        Yaml::Real(r) => r
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(serde_json::Value::Number)
            .unwrap_or_else(|| r.clone().into()),
        Yaml::Array(items) => items.iter().map(yaml_to_json).collect(),
        Yaml::Hash(h) => serde_json::Value::Object(
            h.iter()
                .filter_map(|(k, v)| yaml_to_string(k).map(|k| (k, yaml_to_json(v))))
                .collect(),
        ),
        // `key:` with nothing after it, and the alias/tag forms the tree
        // never uses.
        _ => serde_json::Value::Null,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAN: &str = "---\nprovenance: authored\nowner: org/founder\nstatus: ready\ntype: initiative\nawaits: [plans/a.md, plans/b.md]\nsubdomains:\n  - problem/x.md\n---\n# Body\n";

    #[test]
    fn scalar_and_lists() {
        let fm = extract(PLAN).unwrap();
        assert!(fm.parsed());
        assert_eq!(fm.get_str("status").as_deref(), Some("ready"));
        assert_eq!(
            fm.get_list("awaits").unwrap(),
            vec!["plans/a.md", "plans/b.md"]
        );
        assert_eq!(fm.get_list("subdomains").unwrap(), vec!["problem/x.md"]);
        assert_eq!(fm.line_of("status"), Some(4));
        assert_eq!(fm.close_line, 9);
    }

    #[test]
    fn lenient_fallback_on_broken_yaml() {
        // A tab makes the YAML unparseable; scalar reads must still work.
        let text =
            "---\nstatus: ready\n\tbroken: [\nowner: org/founder\nawaits:\n  - plans/a.md\n---\n";
        let fm = extract(text).unwrap();
        assert!(!fm.parsed());
        assert_eq!(fm.get_str("status").as_deref(), Some("ready"));
        assert_eq!(fm.get_str("owner").as_deref(), Some("org/founder"));
        assert_eq!(fm.get_list("awaits").unwrap(), vec!["plans/a.md"]);
    }

    #[test]
    fn edges() {
        let text = "---\ninduced-by:\n  - strategy: strategy/s.md\n    class: core\n---\n";
        let fm = extract(text).unwrap();
        let edges = fm.get_edges("induced-by", "strategy").unwrap();
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, "strategy/s.md");
        assert_eq!(edges[0].class.as_deref(), Some("core"));
    }

    #[test]
    fn no_frontmatter() {
        assert!(extract("# Just a doc\n").is_none());
        assert!(extract("---\nunclosed: true\n").is_none());
    }
}
