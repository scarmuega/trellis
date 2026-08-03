//! `rituals.md` is the heartbeat; workflow crons are its binding. This
//! module maps cadences to cron expressions, checks a workflow file for
//! drift, and emits a schedule block — keeping "keep the cron in step with
//! rituals.md" a command instead of a chore.

use serde::Serialize;

use crate::registries::Registries;

/// 09:00 UTC, matching the reference workflows.
pub fn expected_cron(cadence: &str) -> Option<String> {
    match cadence.trim().to_lowercase().as_str() {
        "daily" => Some("0 9 * * *".into()),
        "weekly" => Some("0 9 * * 1".into()),
        "biweekly" | "fortnightly" => None, // no clean cron; flag for manual wiring
        "monthly" => Some("0 9 1 * *".into()),
        "quarterly" => Some("0 9 1 1,4,7,10 *".into()),
        _ => None,
    }
}

#[derive(Debug, Serialize)]
pub struct CronCheck {
    pub ritual: String,
    pub cadence: String,
    pub expected_cron: Option<String>,
    /// Whether the workflow declares the expected cron.
    pub satisfied: Option<bool>,
}

pub fn workflow_crons(workflow_text: &str) -> Vec<String> {
    let re = regex::Regex::new(r#"cron:\s*["']([^"']+)["']"#).unwrap();
    re.captures_iter(workflow_text)
        .map(|c| c[1].to_string())
        .collect()
}

pub fn check(reg: &Registries, workflow_text: &str) -> Vec<CronCheck> {
    let crons = workflow_crons(workflow_text);
    reg.rituals
        .iter()
        .map(|r| {
            let expected = expected_cron(&r.cadence);
            CronCheck {
                ritual: r.name.clone(),
                cadence: r.cadence.clone(),
                satisfied: expected.as_ref().map(|e| crons.iter().any(|c| c == e)),
                expected_cron: expected,
            }
        })
        .collect()
}

/// A GitHub `on.schedule` block per distinct cadence, with the rituals it
/// covers named alongside.
pub fn emit_github(reg: &Registries) -> String {
    let mut out = String::from("on:\n  schedule:\n");
    let mut seen: Vec<(String, Vec<String>)> = Vec::new();
    for r in &reg.rituals {
        let Some(cron) = expected_cron(&r.cadence) else {
            continue;
        };
        match seen.iter_mut().find(|(c, _)| *c == cron) {
            Some((_, names)) => names.push(r.name.clone()),
            None => seen.push((cron, vec![r.name.clone()])),
        }
    }
    for (cron, names) in seen {
        out.push_str(&format!("    - cron: \"{cron}\" # {}\n", names.join(", ")));
    }
    out
}
