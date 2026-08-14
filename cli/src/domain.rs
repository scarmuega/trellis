//! `trellis.toml` — the domain's machine-read configuration, and the root
//! marker (decision 0054).
//!
//! Domain-owned, never binding-owned: the spec pin, the open registries, the
//! carried-content prefixes, and the retention horizon are governance the
//! whole domain agrees on, versioned with the root and identical in every
//! clone — where `runtime.toml` belongs to whoever runs the daemon. The
//! values are for agents as much as for the kernel: registry entries map a
//! name to the one-line definition a session reads when choosing it; the
//! kernel consumes only the keys.
//!
//! Unlike `runtime.toml`, an absent file is never legal here — it is what
//! makes the directory a root — and a file that does not parse (unknown keys
//! included) refuses the whole load rather than shrinking to defaults: a
//! lint pass over empty registries would report violations that are not
//! there, with deterministic authority (decision 0037).

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

pub const FILE: &str = "trellis.toml";

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DomainConfig {
    /// The spec version this root operates against. Required; lint item 18
    /// compares it to the kernel's own.
    pub spec: u32,
    /// Carried-content registry: root-relative path prefixes this root
    /// carries but does not author (decision 0040). Walked and secret-swept,
    /// never artifacts.
    #[serde(default)]
    pub carried: Vec<String>,
    /// Plan-type registry: open set, name → one-line definition (lint item 4).
    #[serde(default)]
    pub plan_types: BTreeMap<String, String>,
    /// Tag registry: name → what the tag groups (lint item 8).
    #[serde(default)]
    pub tags: BTreeMap<String, String>,
    /// Decision registry: path → disposition note for prose-era
    /// supersessions and inert classifications (lint item 26).
    #[serde(default)]
    pub decisions: BTreeMap<String, String>,
    #[serde(default)]
    pub archive: Archive,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Archive {
    /// Days a terminal status must hold before the archive sweep files the
    /// artifact. Absent = the sweep refuses, as ever: a horizon nobody
    /// declared is not one the kernel invents.
    pub after_days: Option<i64>,
}

impl DomainConfig {
    /// Read `{root}/trellis.toml`. Absence is an error — the file is the
    /// root marker, so a caller holding a discovered root that then finds no
    /// file is racing a deletion, and says so plainly.
    pub fn load(root_path: &Path) -> anyhow::Result<DomainConfig> {
        let path = root_path.join(FILE);
        let text = std::fs::read_to_string(&path)
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        let cfg: DomainConfig =
            basic_toml::from_str(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        cfg.validate()
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        Ok(cfg)
    }

    /// Everything worth refusing at load rather than mid-command.
    fn validate(&self) -> anyhow::Result<()> {
        if self.spec == 0 {
            anyhow::bail!("spec = 0 — the pin names the spec version this root operates against");
        }
        if let Some(days) = self.archive.after_days {
            if days < 1 {
                anyhow::bail!(
                    "archive.after_days = {days} — the retention horizon is at least one day"
                );
            }
        }
        if self.carried.iter().any(|p| p.trim().is_empty()) {
            anyhow::bail!("carried lists an empty prefix — an empty prefix would carry the root");
        }
        for path in self.decisions.keys() {
            if !path.starts_with("decisions/") {
                anyhow::bail!(
                    "[decisions] key {path} does not start with decisions/ — entries name \
                     decision artifacts by root-relative path"
                );
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> anyhow::Result<DomainConfig> {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(FILE), text).unwrap();
        DomainConfig::load(dir.path())
    }

    #[test]
    fn a_bare_pin_is_a_complete_config() {
        let cfg = parse("spec = 21\n").unwrap();
        assert_eq!(cfg.spec, 21);
        assert!(cfg.carried.is_empty());
        assert!(cfg.plan_types.is_empty());
        assert!(cfg.archive.after_days.is_none());
    }

    #[test]
    fn an_absent_file_is_an_error_not_a_default() {
        let dir = tempfile::TempDir::new().unwrap();
        let err = DomainConfig::load(dir.path()).unwrap_err();
        assert!(err.to_string().contains(FILE), "{err}");
    }

    #[test]
    fn a_missing_pin_refuses_by_field_name() {
        let err = parse("[tags]\n").unwrap_err();
        assert!(err.to_string().contains("spec"), "{err}");
    }

    #[test]
    fn a_misspelled_key_refuses_rather_than_being_ignored() {
        let err = parse("spec = 21\nspeling = 1\n").unwrap_err();
        assert!(err.to_string().contains("speling"), "{err}");
    }

    #[test]
    fn registries_map_names_to_definitions() {
        let cfg = parse(
            "spec = 21\ncarried = [\"solution/kit/storefront/\"]\n\n\
             [plan_types]\ninitiative = \"general time-bounded execution\"\n\n\
             [decisions]\n\"decisions/0003-x.md\" = \"inert: scaffolding era\"\n\n\
             [archive]\nafter_days = 90\n",
        )
        .unwrap();
        assert_eq!(cfg.plan_types["initiative"], "general time-bounded execution");
        assert_eq!(cfg.carried, vec!["solution/kit/storefront/"]);
        assert_eq!(cfg.decisions["decisions/0003-x.md"], "inert: scaffolding era");
        assert_eq!(cfg.archive.after_days, Some(90));
    }

    #[test]
    fn a_decision_entry_outside_decisions_is_refused() {
        let err = parse("spec = 21\n[decisions]\n\"plans/x.md\" = \"inert\"\n").unwrap_err();
        assert!(err.to_string().contains("plans/x.md"), "{err}");
    }

    #[test]
    fn a_zero_horizon_is_refused() {
        let err = parse("spec = 21\n[archive]\nafter_days = 0\n").unwrap_err();
        assert!(err.to_string().contains("after_days"), "{err}");
    }
}
