//! Instance registries, read at runtime and never hardcoded: plan types and
//! tags from `conventions.md`, ritual cadences (and the actuals freshness
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
    pub rituals: Vec<Ritual>,
    /// Days, when derivable (metric-sweep cadence, or an explicit
    /// "freshness window … N days" statement in `rituals.md`).
    pub freshness_window_days: Option<i64>,
    /// Days a terminal artifact waits before the sweep files it into the
    /// tier, from an "archive after … N days" statement in `conventions.md`.
    /// Absent means the sweep moves nothing on its own: a retention horizon
    /// nobody declared is not one the kernel invents.
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
    let mut reg = Registries::default();

    if let Some(conv) = tree.get("conventions.md") {
        reg.plan_types = markdown::registry_items(&conv.text, &conv.headings, "plan-type-registry");
        reg.tags = markdown::registry_items(&conv.text, &conv.headings, "tag-registry");
        let re = regex::Regex::new(r"archive after[^.\n]*?(\d+)\s*days").unwrap();
        if let Some(cap) = re.captures(&conv.text) {
            reg.archive_after_days = cap[1].parse().ok();
        }
    }

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
