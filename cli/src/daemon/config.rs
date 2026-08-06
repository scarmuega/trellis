//! `runtime.toml` — the local binding's configuration, read from the domain
//! root.
//!
//! Binding-owned, never model-owned: the harness command templates and the
//! complexity→session map live with whoever runs the scan (decision 0032), and
//! since the daemon runs it, this file is where they live. Every field has a
//! default, so a root with no `runtime.toml` still serves. Secrets never live
//! here — the daemon's own environment carries them and spawned sessions
//! inherit it.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::tmpl;
use crate::dispatch::SessionMap;

pub const FILE: &str = "runtime.toml";

#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RuntimeConfig {
    pub server: Server,
    pub scheduler: Scheduler,
    pub harness: Harness,
    pub prompts: Prompts,
    pub sessions: Sessions,
    /// Reserved for push transports. The adapter seam exists; no adapter
    /// does (decision 0038), so a populated table is refused rather than
    /// quietly ignored.
    pub channels: Vec<ReservedChannel>,
}

/// A `[[channels]]` entry, parsed only far enough to name itself in the
/// refusal message.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReservedChannel {
    pub kind: String,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Server {
    pub bind: String,
    /// 0 asks the OS for a port, which the daemon prints as it starts.
    pub port: u16,
}

impl Default for Server {
    fn default() -> Self {
        Server {
            bind: "127.0.0.1".into(),
            port: 7357,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Catchup {
    /// A cadence missed while the machine slept fires once on waking.
    Once,
    /// A missed cadence waits for its next natural window.
    Skip,
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Scheduler {
    pub tick_secs: u64,
    pub max_concurrent: usize,
    /// Overrides the `plan dispatch` row's cadence in `rituals.md`.
    pub dispatch_cadence: Option<String>,
    pub catchup: Catchup,
}

impl Default for Scheduler {
    fn default() -> Self {
        Scheduler {
            tick_secs: 60,
            max_concurrent: 2,
            dispatch_cadence: None,
            catchup: Catchup::Once,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Harness {
    /// argv for one `act` session. Claude Code is the reference adapter;
    /// another harness is this array, rewritten.
    ///
    /// The defaults name no `{plugin_dir}`: the daemon runs on a machine
    /// where the plugin is installed in the harness, which is the whole
    /// premise of operating locally. A checkout that is *not* installed —
    /// running against a working tree, say — adds `--plugin-dir
    /// {plugin_dir}` here and supplies it with `--plugin-root` or
    /// `CLAUDE_PLUGIN_ROOT`.
    ///
    /// They do name `{mcp}`: the session's back-channel, so it can ask a
    /// question rather than block its plan (decision 0041). A harness that
    /// does not speak MCP drops that pair of arguments and everything else
    /// works — the channel is opt-in per binding, not a requirement of one.
    pub act_cmd: Vec<String>,
    /// argv for one ritual session.
    pub ritual_cmd: Vec<String>,
}

impl Default for Harness {
    fn default() -> Self {
        let argv = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect();
        Harness {
            act_cmd: argv(&[
                "claude",
                "-p",
                "{prompt}",
                "--permission-mode",
                "auto",
                "--model",
                "{model}",
                "--effort",
                "{effort}",
                "--max-budget-usd",
                "{budget}",
                "--mcp-config",
                "{mcp}",
            ]),
            ritual_cmd: argv(&[
                "claude",
                "-p",
                "{prompt}",
                "--permission-mode",
                "acceptEdits",
                "--allowedTools",
                "Read",
                "Grep",
                "Glob",
                "Write",
                "Edit",
                "Bash(gh *)",
                "Bash(git *)",
                "--max-budget-usd",
                "5",
                "--mcp-config",
                "{mcp}",
            ]),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Prompts {
    pub act: String,
    pub ritual: String,
}

impl Default for Prompts {
    fn default() -> Self {
        Prompts {
            act: "/trellis:act {owner} advance {plan} toward its objective: make the next \
                  increment of progress within your authority; flip its status ready→active \
                  as you claim it, blocked if you hit an uncleared blocker; leave a trail."
                .into(),
            ritual: "/trellis:ritual {ritual}".into(),
        }
    }
}

/// `model:effort:budget` per complexity tier — the same triples
/// `dispatch scan --map` takes.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Sessions {
    pub mechanical: Option<String>,
    pub standard: Option<String>,
    pub deep: Option<String>,
}

impl RuntimeConfig {
    /// Read `{root}/runtime.toml`, or an explicit path. An absent default
    /// file is legal — an absent *explicit* one is not.
    pub fn load(root: &Path, explicit: Option<&Path>) -> anyhow::Result<RuntimeConfig> {
        let (path, required) = match explicit {
            Some(p) => (p.to_path_buf(), true),
            None => (root.join(FILE), false),
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound && !required => {
                return Ok(RuntimeConfig::default())
            }
            Err(e) => return Err(anyhow::anyhow!("{}: {e}", path.display())),
        };
        let cfg: RuntimeConfig =
            basic_toml::from_str(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        cfg.validate()
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        Ok(cfg)
    }

    /// Everything worth refusing before the first tick rather than at the
    /// first spawn.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.harness.act_cmd.is_empty() {
            anyhow::bail!("harness.act_cmd is empty — there is no command to run a session with");
        }
        if self.harness.ritual_cmd.is_empty() {
            anyhow::bail!("harness.ritual_cmd is empty — there is no command to run a ritual with");
        }
        tmpl::check("harness.act_cmd", &self.harness.act_cmd)?;
        tmpl::check("harness.ritual_cmd", &self.harness.ritual_cmd)?;
        tmpl::check("prompts.act", std::slice::from_ref(&self.prompts.act))?;
        tmpl::check("prompts.ritual", std::slice::from_ref(&self.prompts.ritual))?;
        if self.scheduler.tick_secs == 0 {
            anyhow::bail!("scheduler.tick_secs must be at least 1");
        }
        if self.scheduler.max_concurrent == 0 {
            anyhow::bail!("scheduler.max_concurrent must be at least 1 — 0 spawns nothing");
        }
        if let Some(c) = &self.channels.first() {
            anyhow::bail!(
                "channels are configured ('{}') but no push adapter ships in this binding — \
                 the seam exists, the transports do not (decision 0038); escalation records \
                 are read from the API, the board, or the root",
                c.kind
            );
        }
        self.session_map()?;
        Ok(())
    }

    /// The complexity→session map, defaults overridden by `[sessions]`.
    pub fn session_map(&self) -> anyhow::Result<SessionMap> {
        let mut map = SessionMap::default();
        for (tier, spec) in [
            ("mechanical", &self.sessions.mechanical),
            ("standard", &self.sessions.standard),
            ("deep", &self.sessions.deep),
        ] {
            if let Some(spec) = spec {
                map.apply(&format!("{tier}={spec}"))?;
            }
        }
        Ok(map)
    }

    /// Where this daemon keeps its ephemeral state: gitignored, never an
    /// artifact, never committed.
    pub fn runtime_dir(root: &Path) -> PathBuf {
        root.join(".trellis").join("runtime")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_file_is_a_default_config() {
        let dir = tempfile::TempDir::new().unwrap();
        let cfg = RuntimeConfig::load(dir.path(), None).unwrap();
        assert_eq!(cfg.server.port, 7357);
        assert_eq!(cfg.harness.act_cmd[0], "claude");
        assert_eq!(cfg.scheduler.catchup, Catchup::Once);
    }

    #[test]
    fn a_named_file_that_is_missing_is_an_error() {
        let dir = tempfile::TempDir::new().unwrap();
        let err = RuntimeConfig::load(dir.path(), Some(&dir.path().join("nope.toml"))).unwrap_err();
        assert!(err.to_string().contains("nope.toml"), "{err}");
    }

    fn parse(text: &str) -> anyhow::Result<RuntimeConfig> {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join(FILE), text).unwrap();
        RuntimeConfig::load(dir.path(), None)
    }

    #[test]
    fn partial_tables_keep_the_untouched_defaults() {
        let cfg = parse("[server]\nport = 9000\n").unwrap();
        assert_eq!(cfg.server.port, 9000);
        assert_eq!(cfg.server.bind, "127.0.0.1");
        assert_eq!(cfg.scheduler.tick_secs, 60);
    }

    #[test]
    fn sessions_override_only_the_named_tier() {
        let cfg = parse("[sessions]\ndeep = \"fable:max:40\"\n").unwrap();
        let map = cfg.session_map().unwrap();
        assert_eq!(map.deep.model, "fable");
        assert_eq!(map.deep.budget_usd, 40.0);
        assert_eq!(map.standard.model, "opus");
    }

    #[test]
    fn a_misspelled_key_refuses_rather_than_being_ignored() {
        let err = parse("[scheduler]\ntick_seconds = 5\n").unwrap_err();
        assert!(err.to_string().contains("tick_seconds"), "{err}");
    }

    #[test]
    fn a_misspelled_placeholder_is_caught_at_load() {
        let err = parse("[harness]\nact_cmd = [\"claude\", \"{promt}\"]\n").unwrap_err();
        assert!(err.to_string().contains("not a placeholder"), "{err}");
    }

    #[test]
    fn a_configured_channel_is_refused_while_no_adapter_ships() {
        let err = parse("[[channels]]\nkind = \"webhook\"\n").unwrap_err();
        assert!(err.to_string().contains("no push adapter"), "{err}");
    }
}
