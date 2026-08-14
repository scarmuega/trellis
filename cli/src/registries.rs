//! Instance registries, read at runtime and never hardcoded: plan types and
//! tags from `trellis.toml`, ritual cadences (and the actuals freshness
//! window) from `rituals.md`.

use crate::markdown;
use crate::tree::Tree;

#[derive(Debug, Clone)]
pub struct Ritual {
    pub name: String,
    pub cadence: String,
    pub executor: String,
    pub procedure: String,
}

#[derive(Debug, Default)]
pub struct Registries {
    pub plan_types: Vec<String>,
    pub tags: Vec<String>,
    /// Decisions the registry carries a disposition for — prose-era
    /// supersessions and inert classifications the frozen files cannot
    /// declare (trellis.toml → `[decisions]`). Entries are the decision
    /// paths; the disposition prose stays in the table's value.
    pub decision_registry: Vec<String>,
    pub rituals: Vec<Ritual>,
    /// Days, when derivable (metric-sweep cadence, or an explicit
    /// "freshness window … N days" statement in `rituals.md`).
    pub freshness_window_days: Option<i64>,
    /// Days a terminal artifact waits before the sweep files it into the
    /// tier, from `archive.after_days` in `trellis.toml`. Absent means the
    /// sweep moves nothing on its own: a retention horizon nobody declared
    /// is not one the kernel invents.
    pub archive_after_days: Option<i64>,
}

pub fn cadence_days(cadence: &str) -> Option<i64> {
    let c = cadence.trim().to_lowercase();
    match c.as_str() {
        "daily" => Some(1),
        "weekly" => Some(7),
        "biweekly" | "fortnightly" => Some(14),
        "monthly" => Some(30),
        "quarterly" => Some(90),
        _ => {
            // "N days" / "every N days"
            let mut last_num: Option<i64> = None;
            for tok in c.split_whitespace() {
                if let Ok(n) = tok.parse::<i64>() {
                    last_num = Some(n);
                }
            }
            if c.contains("day") {
                last_num
            } else if c.contains("week") {
                last_num.map(|n| n * 7)
            } else {
                None
            }
        }
    }
}

pub fn load(tree: &Tree) -> Registries {
    // The machine-read half of the domain's config: parsed and validated
    // before the tree existed, copied here so every consumer keeps one type.
    let mut reg = Registries {
        plan_types: tree.domain.plan_types.keys().cloned().collect(),
        tags: tree.domain.tags.keys().cloned().collect(),
        decision_registry: tree.domain.decisions.keys().cloned().collect(),
        archive_after_days: tree.domain.archive.after_days,
        ..Registries::default()
    };

    if let Some(rituals) = tree.get("rituals.md") {
        let skip = rituals.fm.as_ref().map(|f| f.close_line).unwrap_or(0);
        for table in markdown::tables(&rituals.text, skip) {
            let header: Vec<String> = table.header.iter().map(|h| h.to_lowercase()).collect();
            let col = |name: &str| header.iter().position(|h| h == name);
            let (Some(ci_name), Some(ci_cad)) = (col("ritual"), col("cadence")) else {
                continue;
            };
            for (cells, _) in &table.rows {
                let get =
                    |i: Option<usize>| i.and_then(|i| cells.get(i)).cloned().unwrap_or_default();
                reg.rituals.push(Ritual {
                    name: get(Some(ci_name)),
                    cadence: get(Some(ci_cad)),
                    executor: get(col("executor")),
                    procedure: get(col("procedure")),
                });
            }
        }

        // Explicit override: "… freshness window … is N days" in prose.
        let re = regex::Regex::new(r"freshness window[^.\n]*?(\d+)\s*days").unwrap();
        if let Some(cap) = re.captures(&rituals.text) {
            reg.freshness_window_days = cap[1].parse().ok();
        } else if let Some(sweep) = reg
            .rituals
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case("metric sweep"))
        {
            reg.freshness_window_days = cadence_days(&sweep.cadence);
        }
    }

    reg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cadences() {
        assert_eq!(cadence_days("daily"), Some(1));
        assert_eq!(cadence_days("weekly"), Some(7));
        assert_eq!(cadence_days("every 3 days"), Some(3));
        assert_eq!(cadence_days("2 weeks"), Some(14));
        assert_eq!(cadence_days("on demand"), None);
    }
}
