//! Line-oriented markdown scanning for the constrained shapes Trellis uses:
//! ATX headings (anchor targets), fenced code blocks (escalation and finding
//! records), and pipe tables (rituals, context map). The domain's markdown is
//! convention-bound; a full CommonMark AST buys nothing here.

#[derive(Debug, Clone)]
pub struct Heading {
    pub level: u8,
    pub text: String,
    pub slug: String,
    pub line: u32,
}

#[derive(Debug, Clone)]
pub struct FencedBlock {
    pub lang: String,
    pub content: String,
    /// 1-based line of the opening fence.
    pub open_line: u32,
    /// Index into the headings vec of the section this block sits under.
    pub section: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub header: Vec<String>,
    /// (cells, 1-based line) per data row.
    pub rows: Vec<(Vec<String>, u32)>,
    pub line: u32,
}

/// GitHub-style anchor slug: lowercase, alphanumerics kept, spaces to
/// hyphens, everything else dropped (hyphens kept).
pub fn slugify(text: &str) -> String {
    let mut out = String::new();
    for c in text.trim().chars() {
        if c.is_alphanumeric() {
            for lc in c.to_lowercase() {
                out.push(lc);
            }
        } else if c == ' ' || c == '-' || c == '_' {
            out.push(if c == ' ' { '-' } else { c });
        }
    }
    out
}

/// Scan full file text. `skip_through` is the last line of the frontmatter
/// block (0 when there is none): YAML comments inside frontmatter would
/// otherwise read as headings.
pub fn scan(text: &str, skip_through: u32) -> (Vec<Heading>, Vec<FencedBlock>) {
    let mut headings = Vec::new();
    let mut fences = Vec::new();
    let mut in_fence = false;
    let mut fence_lang = String::new();
    let mut fence_content = String::new();
    let mut fence_open: u32 = 0;

    for (i, line) in text.lines().enumerate() {
        let line_no = i as u32 + 1;
        if line_no <= skip_through {
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            if in_fence {
                fences.push(FencedBlock {
                    lang: fence_lang.clone(),
                    content: std::mem::take(&mut fence_content),
                    open_line: fence_open,
                    section: if headings.is_empty() {
                        None
                    } else {
                        Some(headings.len() - 1)
                    },
                });
                in_fence = false;
            } else {
                in_fence = true;
                fence_lang = trimmed.trim_start_matches('`').trim().to_string();
                fence_open = line_no;
            }
            continue;
        }
        if in_fence {
            fence_content.push_str(line);
            fence_content.push('\n');
            continue;
        }
        if let Some(rest) = line.strip_prefix('#') {
            let mut level = 1u8;
            let mut rest = rest;
            while let Some(r) = rest.strip_prefix('#') {
                level += 1;
                rest = r;
            }
            if let Some(text) = rest.strip_prefix(' ') {
                let text = text.trim().to_string();
                headings.push(Heading {
                    level,
                    slug: slugify(&text),
                    text,
                    line: line_no,
                });
            }
        }
    }
    (headings, fences)
}

/// Parse every pipe table outside fences.
pub fn tables(text: &str, skip_through: u32) -> Vec<Table> {
    let mut out: Vec<Table> = Vec::new();
    let mut in_fence = false;
    let mut current: Option<Table> = None;
    let mut awaiting_separator = false;

    for (i, line) in text.lines().enumerate() {
        let line_no = i as u32 + 1;
        if line_no <= skip_through {
            continue;
        }
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let t = line.trim();
        if t.starts_with('|') && t.len() > 1 {
            let cells: Vec<String> = t
                .trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect();
            let is_separator = cells
                .iter()
                .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':' || ch == ' '));
            match (&mut current, awaiting_separator, is_separator) {
                (None, _, false) => {
                    current = Some(Table {
                        header: cells,
                        rows: vec![],
                        line: line_no,
                    });
                    awaiting_separator = true;
                }
                (Some(_), true, true) => awaiting_separator = false,
                (Some(t), false, false) => t.rows.push((cells, line_no)),
                _ => {}
            }
        } else if let Some(t) = current.take() {
            if !t.rows.is_empty() {
                out.push(t);
            }
            awaiting_separator = false;
        }
    }
    if let Some(t) = current {
        if !t.rows.is_empty() {
            out.push(t);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs() {
        assert_eq!(slugify("N-trail-nutrition"), "n-trail-nutrition");
        assert_eq!(slugify("Plan-type registry"), "plan-type-registry");
        assert_eq!(slugify("Booking Throughput"), "booking-throughput");
    }

    #[test]
    fn headings_skip_fences_and_frontmatter() {
        let text = "---\n# yaml comment\n---\n# Title\n```yaml\n# not a heading\nkind: gap\n```\n## Escalations\n";
        let (h, f) = scan(text, 3);
        assert_eq!(h.len(), 2);
        assert_eq!(h[0].text, "Title");
        assert_eq!(h[1].slug, "escalations");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].lang, "yaml");
        assert!(f[0].content.contains("kind: gap"));
        assert_eq!(f[0].section, Some(0));
    }

    #[test]
    fn table_parse() {
        let text = "| ritual | cadence |\n|--------|---------|\n| lint   | weekly  |\n| focus  | weekly  |\n";
        let t = tables(text, 0);
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].header, vec!["ritual", "cadence"]);
        assert_eq!(t[0].rows.len(), 2);
        assert_eq!(t[0].rows[0].0, vec!["lint", "weekly"]);
    }

}
