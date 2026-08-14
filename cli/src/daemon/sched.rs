//! Which rituals are due, computed from declared cadences and last-fired
//! dates alone. Pure: no clock, no filesystem, no spawning — the caller
//! supplies today and acts on the answer, which is what makes cadence
//! behaviour testable without waiting for one.
//!
//! Granularity is a day, matching `rituals.md`'s vocabulary. A daily ritual
//! fires when the runner first notices a new day, not at a wall-clock hour:
//! the forge binding's crons carry an hour because cron demands one, and the
//! cadence never did.
//!
//! Plan dispatch is not here (decision 0046). A ritual's time gap is part of
//! its meaning — the metric sweep's cadence *is* the freshness window — but
//! dispatch's daily cadence was a fossil of the forge cron it was ported
//! from: nothing about "daily" means anything to a scan whose idempotency
//! comes from claims. The dispatcher polls every tick; this module schedules
//! only the sessions whose gaps are semantic.

use super::config::{Catchup, RuntimeConfig};
use super::state::{self, State};
use crate::dates::{self, Date};
use crate::registries::{cadence_days, Registries};

/// The `rituals.md` row whose standing behaviour is the dispatcher's loop.
/// It stays in the table as the operator-of-record declaration, and the
/// scheduler skips it: the loop is continuous, not cadenced.
pub const DISPATCH_ROW: &str = "plan dispatch";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub name: String,
    pub executor: String,
}

impl Task {
    pub fn key(&self) -> String {
        state::key_ritual(&self.name)
    }

    pub fn label(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Due {
    pub task: Task,
    /// A window missed while nothing was running, under `catchup = "skip"`:
    /// the run is recorded and nothing is spawned, so the next fire lands a
    /// full period from now instead of immediately.
    pub skipped: bool,
}

#[derive(Debug, Default)]
pub struct Pass {
    pub due: Vec<Due>,
    /// Rows whose cadence has no day count ("on demand", "biweekly" phrased
    /// unusually). Never fired automatically; reported so the silence is
    /// visible rather than assumed.
    pub unscheduled: Vec<String>,
}

/// Every ritual due as of `today`.
pub fn due(reg: &Registries, cfg: &RuntimeConfig, state: &State, today: Date) -> Pass {
    let mut pass = Pass::default();

    for row in &reg.rituals {
        if row.name.eq_ignore_ascii_case(DISPATCH_ROW) {
            continue; // the dispatcher's loop, never a ritual session
        }
        let task = Task {
            name: row.name.clone(),
            executor: row.executor.clone(),
        };
        match cadence_days(&row.cadence) {
            None => pass.unscheduled.push(row.name.clone()),
            Some(days) => consider(&mut pass, task, days, cfg, state, today),
        }
    }

    pass
}

fn consider(
    pass: &mut Pass,
    task: Task,
    cadence_days: i64,
    cfg: &RuntimeConfig,
    state: &State,
    today: Date,
) {
    let Some(last) = state.last_fired(&task.key()) else {
        // Never fired: nothing was missed, so both policies run it.
        pass.due.push(Due {
            task,
            skipped: false,
        });
        return;
    };
    let elapsed = dates::days_between(last, today);
    if elapsed < cadence_days {
        return;
    }
    let late = elapsed > cadence_days;
    pass.due.push(Due {
        skipped: late && cfg.scheduler.catchup == Catchup::Skip,
        task,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registries::Ritual;

    fn reg(rows: &[(&str, &str)]) -> Registries {
        Registries {
            rituals: rows
                .iter()
                .map(|(name, cadence)| Ritual {
                    name: (*name).into(),
                    cadence: (*cadence).into(),
                    executor: "org/steward".into(),
                    procedure: String::new(),
                })
                .collect(),
            ..Registries::default()
        }
    }

    fn day(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    fn labels(pass: &Pass) -> Vec<&str> {
        pass.due.iter().map(|d| d.task.label()).collect()
    }

    #[test]
    fn an_unfired_ritual_is_due_immediately() {
        let pass = due(
            &reg(&[("conventions lint", "weekly")]),
            &RuntimeConfig::default(),
            &State::default(),
            day("2026-08-03"),
        );
        assert_eq!(labels(&pass), vec!["conventions lint"]);
        assert!(pass.due.iter().all(|d| !d.skipped));
    }

    #[test]
    fn a_ritual_fired_today_is_not_due_again() {
        let reg = reg(&[("conventions lint", "weekly")]);
        let mut state = State::default();
        state.fired(
            &state::key_ritual("conventions lint"),
            "ritual",
            day("2026-08-03"),
            None,
        );
        let pass = due(&reg, &RuntimeConfig::default(), &state, day("2026-08-03"));
        assert!(pass.due.is_empty(), "{:?}", labels(&pass));
    }

    #[test]
    fn the_cadence_decides_when_it_returns() {
        let reg = reg(&[("focus", "weekly")]);
        let mut state = State::default();
        state.fired(
            &state::key_ritual("focus"),
            "ritual",
            day("2026-08-03"),
            None,
        );
        let cfg = RuntimeConfig::default();

        // Day 6: the weekly row waits.
        let pass = due(&reg, &cfg, &state, day("2026-08-09"));
        assert!(labels(&pass).is_empty());

        // Day 7: exactly on cadence.
        let pass = due(&reg, &cfg, &state, day("2026-08-10"));
        assert!(labels(&pass).contains(&"focus"));
    }

    #[test]
    fn an_overdue_window_fires_once_not_once_per_window() {
        let reg = reg(&[("focus", "weekly")]);
        let mut state = State::default();
        state.fired(
            &state::key_ritual("focus"),
            "ritual",
            day("2026-06-01"),
            None,
        );
        // Two months late — still exactly one due entry.
        let pass = due(&reg, &RuntimeConfig::default(), &state, day("2026-08-03"));
        assert_eq!(
            pass.due
                .iter()
                .filter(|d| d.task.label() == "focus")
                .count(),
            1
        );
    }

    #[test]
    fn catchup_skip_records_the_missed_window_without_running_it() {
        let reg = reg(&[("focus", "weekly")]);
        let mut state = State::default();
        state.fired(
            &state::key_ritual("focus"),
            "ritual",
            day("2026-06-01"),
            None,
        );
        let cfg = RuntimeConfig {
            scheduler: super::super::config::Scheduler {
                catchup: Catchup::Skip,
                ..Default::default()
            },
            ..Default::default()
        };
        let pass = due(&reg, &cfg, &state, day("2026-08-03"));
        let focus = pass
            .due
            .iter()
            .find(|d| d.task.label() == "focus")
            .expect("still reported");
        assert!(focus.skipped);
    }

    #[test]
    fn a_cadence_with_no_day_count_never_fires_and_says_so() {
        let pass = due(
            &reg(&[("incident review", "on demand")]),
            &RuntimeConfig::default(),
            &State::default(),
            day("2026-08-03"),
        );
        assert_eq!(pass.unscheduled, vec!["incident review"]);
        assert!(labels(&pass).is_empty());
    }

    #[test]
    fn the_dispatch_row_is_the_dispatchers_loop_never_a_ritual_session() {
        let pass = due(
            &reg(&[("plan dispatch", "daily")]),
            &RuntimeConfig::default(),
            &State::default(),
            day("2026-08-03"),
        );
        assert!(pass.due.is_empty());
        assert!(pass.unscheduled.is_empty());
    }
}
