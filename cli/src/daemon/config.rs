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
    /// Push transports for newly opened escalation records. The seam is
    /// 0038's; the one adapter that ships is `kind = "herdr"` (decision
    /// 0043). Any other kind is still refused rather than quietly ignored.
    pub channels: Vec<ChannelCfg>,
}

/// One `[[channels]]` entry.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChannelCfg {
    pub kind: String,
    /// The herdr socket to notify, when not herdr's own default.
    pub socket: Option<String>,
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
    /// Seconds between the dispatcher's passes.
    pub tick_secs: u64,
    /// Sessions one process may run at once — dispatch and rituals each get
    /// their own allowance (decision 0046), so a ritual batch cannot starve
    /// dispatch.
    pub max_concurrent: usize,
    /// Pre-0046: overrode the `plan dispatch` row's cadence. Dispatch has no
    /// cadence anymore — the loop polls every tick — so the field is parsed
    /// for compatibility and noted as obsolete at startup.
    pub dispatch_cadence: Option<String>,
    /// Rituals only: what a cadence missed while nothing was running does.
    pub catchup: Catchup,
    /// Dispatch only: seconds a plan is skipped after a session ended
    /// without claiming it. The daily cadence used to be this backstop by
    /// accident; a tight loop needs it on purpose, or a crash-looping act
    /// respawns every tick.
    pub retry_cooldown_secs: u64,
}

impl Default for Scheduler {
    fn default() -> Self {
        Scheduler {
            tick_secs: 60,
            max_concurrent: 2,
            dispatch_cadence: None,
            catchup: Catchup::Once,
            retry_cooldown_secs: 900,
        }
    }
}

/// Who carries a running session (decision 0043).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BackendKind {
    /// The daemon spawns headless one-shot children itself — the default,
    /// and the only backend with exit codes and budget caps.
    Process,
    /// Sessions run as interactive agents in herdr panes: attachable,
    /// visible, restart-durable — and budget-uncapped, which is the trade
    /// 0043 records.
    Herdr,
}

/// What a finished herdr session's workspace does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Retain {
    /// Keep the scene of a failure for attach; close the clean ones.
    OnFailure,
    Always,
    Never,
}

/// What daemon shutdown does to herdr sessions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnShutdown {
    /// Leave them running; the next daemon adopts them.
    Detach,
    /// Close their workspaces, matching the process backend's stop.
    Stop,
}

/// The herdr half of the harness: agent flags, not a full command — herdr
/// owns the executable (`kind`), and the prompt is submitted to the running
/// TUI rather than carried in argv.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HerdrHarness {
    /// A herdr-supported agent kind (its canonical executable).
    pub kind: String,
    /// The herdr socket, when not herdr's own default.
    pub socket: Option<String>,
    pub act_args: Vec<String>,
    pub ritual_args: Vec<String>,
    pub retain: Retain,
    pub on_shutdown: OnShutdown,
}

impl Default for HerdrHarness {
    fn default() -> Self {
        let argv = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect();
        HerdrHarness {
            kind: "claude".into(),
            socket: None,
            act_args: argv(&[
                "--permission-mode",
                "auto",
                "--model",
                "{model}",
                "--effort",
                "{effort}",
                "--mcp-config",
                "{mcp}",
            ]),
            ritual_args: argv(&[
                "--permission-mode",
                "acceptEdits",
                "--mcp-config",
                "{mcp}",
            ]),
            retain: Retain::OnFailure,
            on_shutdown: OnShutdown::Detach,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Harness {
    /// Who carries sessions: "process" (default) or "herdr" (decision 0043).
    pub backend: BackendKind,
    /// The herdr backend's own knobs; ignored under "process".
    pub herdr: HerdrHarness,
    /// argv for one `act` session. Claude Code is the reference adapter;
    /// another harness is this array, rewritten.
    ///
    /// The defaults name no `{plugin_dir}`: the default prompts render the
    /// act procedure into the prompt itself (decision 0050), so the spawned
    /// session needs no installed plugin. `--plugin-dir {plugin_dir}` is
    /// still how an instance points sessions at an uninstalled checkout's
    /// hooks and skills — supplied with `--plugin-root` or
    /// `CLAUDE_PLUGIN_ROOT`, which also swap the procedure source.
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
            backend: BackendKind::Process,
            herdr: HerdrHarness::default(),
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

/// The three prompt templates, one per spawn kind. The defaults are
/// self-contained (decision 0050): a per-session-unique first line — the
/// herdr backend proves delivery by matching the prompt's opening against
/// pane scrollback, so the first line must discriminate — the computed facts
/// (decision 0045), and `{procedure}`, the canonical `commands/*.md` body
/// rendered by the daemon. The contract still lives in those files, not
/// here; an instance preferring the installed-plugin spelling sets these
/// back to `/trellis:act {owner} …` and everything renders as before.
#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Prompts {
    pub act: String,
    pub ritual: String,
    /// One refine session (decision 0048). The refinement contract lives in
    /// `commands/refine.md` — carried here only as its `{procedure}`
    /// rendering.
    pub refine: String,
}

impl Default for Prompts {
    fn default() -> Self {
        Prompts {
            act: "{plan} — dispatched act as {owner} (trellis runtime).\n\
                  \n\
                  Act as {owner}: advance {plan} toward its objective — make the next \
                  increment of progress within your authority. The runtime has \
                  pre-verified the mechanical readiness items and this plan's holds, the \
                  acting-role marker is already stamped (leave it alone), and the change \
                  mechanics are computed: {automation}; core never lands, it is proposed. \
                  Evaluate only the judgment items. Claim with `trellis plan claim \
                  {plan}`; on an uncleared blocker `trellis plan block {plan} --by \
                  {owner} --asks …` (escalations go to {escalate_to}); write escalation \
                  records with `trellis escalate add`; leave a trail.\n\
                  \n\
                  The procedure below is the trellis framework's commands/act.md, \
                  rendered into this prompt so no installed plugin is required; the \
                  domain root's conventions.md is authoritative over it where they \
                  differ.\n\
                  \n\
                  {procedure}"
                .into(),
            ritual: "ritual {ritual} — executed by {executor} (trellis runtime).\n\
                  \n\
                  Execute the ritual \"{ritual}\" from rituals.md as {executor}. The \
                  runtime resolved the executor from the row and stamped the acting-role \
                  marker (leave it alone); escalations go to {escalate_to}. Read the row \
                  for the procedure and its cadence — the freshness window for any \
                  metrics involved.\n\
                  \n\
                  The procedure below is the trellis framework's commands/ritual.md with \
                  the commands/act.md procedure it delegates to, rendered into this \
                  prompt so no installed plugin is required; the domain root's \
                  conventions.md is authoritative over it where they differ.\n\
                  \n\
                  {procedure}"
                .into(),
            refine: "refine {plan} as {owner} (trellis runtime).\n\
                  \n\
                  Act as {owner}: refine {plan} — reshape its content, never execute it. \
                  The operator's instruction, verbatim: {instruction}\n\
                  \n\
                  The runtime has already resolved you as the plan's owner and verified \
                  the plan is live and owned; the acting-role marker is already stamped \
                  (leave it alone); escalations go to {escalate_to}.\n\
                  \n\
                  The procedure below is the trellis framework's commands/refine.md with \
                  the commands/act.md procedure it delegates to, rendered into this \
                  prompt so no installed plugin is required; the domain root's \
                  conventions.md is authoritative over it where they differ.\n\
                  \n\
                  {procedure}"
                .into(),
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
        tmpl::check("prompts.refine", std::slice::from_ref(&self.prompts.refine))?;
        if self.scheduler.tick_secs == 0 {
            anyhow::bail!("scheduler.tick_secs must be at least 1");
        }
        if self.scheduler.max_concurrent == 0 {
            anyhow::bail!("scheduler.max_concurrent must be at least 1 — 0 spawns nothing");
        }
        if self.harness.backend == BackendKind::Herdr {
            self.validate_herdr()?;
        }
        if let Some(c) = self.channels.iter().find(|c| c.kind != "herdr") {
            anyhow::bail!(
                "channel kind '{}' has no adapter in this binding — \"herdr\" is the one \
                 that ships (decision 0043); escalation records are always readable from \
                 the API, the board, or the root",
                c.kind
            );
        }
        self.session_map()?;
        Ok(())
    }

    /// What the herdr backend refuses before the first tick. The themes: the
    /// prompt is not an argument there, and a budget would be a lie.
    fn validate_herdr(&self) -> anyhow::Result<()> {
        if self.harness.herdr.kind.is_empty() {
            anyhow::bail!("harness.herdr.kind is empty — there is no agent to start");
        }
        for (name, args) in [
            ("harness.herdr.act_args", &self.harness.herdr.act_args),
            ("harness.herdr.ritual_args", &self.harness.herdr.ritual_args),
        ] {
            tmpl::check(name, args)?;
            for arg in args {
                if arg == "-p" || arg == "--print" {
                    anyhow::bail!(
                        "{name} names {arg}, the headless mode — the herdr backend drives \
                         an interactive agent; headless sessions are backend = \"process\""
                    );
                }
                if arg.contains("{prompt}") {
                    anyhow::bail!(
                        "{name} names {{prompt}} — under herdr the prompt is submitted to \
                         the running agent, never carried as an argument; drop it"
                    );
                }
                if arg.contains("{budget}") || arg == "--max-budget-usd" {
                    anyhow::bail!(
                        "{name} names a budget, and --max-budget-usd only works with \
                         --print — an interactive session cannot enforce one, so naming it \
                         would be silently unenforced; the herdr backend runs uncapped \
                         (decision 0043)"
                    );
                }
            }
        }
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
    fn a_channel_kind_with_no_adapter_is_refused_by_name() {
        let err = parse("[[channels]]\nkind = \"webhook\"\n").unwrap_err();
        assert!(err.to_string().contains("webhook"), "{err}");
        assert!(err.to_string().contains("herdr"), "{err}");
    }

    #[test]
    fn the_herdr_channel_kind_parses() {
        let cfg = parse("[[channels]]\nkind = \"herdr\"\n").unwrap();
        assert_eq!(cfg.channels.len(), 1);
        assert_eq!(cfg.channels[0].kind, "herdr");
        assert!(cfg.channels[0].socket.is_none());
    }

    #[test]
    fn the_default_backend_is_process_and_herdr_has_defaults() {
        let cfg = parse("").unwrap();
        assert_eq!(cfg.harness.backend, BackendKind::Process);
        assert_eq!(cfg.harness.herdr.kind, "claude");
        assert_eq!(cfg.harness.herdr.retain, Retain::OnFailure);
        assert_eq!(cfg.harness.herdr.on_shutdown, OnShutdown::Detach);
        assert!(!cfg.harness.herdr.act_args.is_empty());
    }

    #[test]
    fn the_herdr_backend_parses_with_its_knobs() {
        let cfg = parse(
            "[harness]\nbackend = \"herdr\"\n\n[harness.herdr]\nretain = \"always\"\non_shutdown = \"stop\"\n",
        )
        .unwrap();
        assert_eq!(cfg.harness.backend, BackendKind::Herdr);
        assert_eq!(cfg.harness.herdr.retain, Retain::Always);
        assert_eq!(cfg.harness.herdr.on_shutdown, OnShutdown::Stop);
    }

    #[test]
    fn a_budget_under_the_herdr_backend_is_refused_as_unenforceable() {
        let err = parse(
            "[harness]\nbackend = \"herdr\"\n\n[harness.herdr]\nact_args = [\"--max-budget-usd\", \"{budget}\"]\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("--print"), "{err}");
        assert!(err.to_string().contains("0043"), "{err}");
    }

    #[test]
    fn print_mode_under_the_herdr_backend_is_refused() {
        let err = parse(
            "[harness]\nbackend = \"herdr\"\n\n[harness.herdr]\nact_args = [\"-p\"]\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("interactive"), "{err}");
    }

    #[test]
    fn a_prompt_placeholder_under_the_herdr_backend_is_refused() {
        let err = parse(
            "[harness]\nbackend = \"herdr\"\n\n[harness.herdr]\nritual_args = [\"{prompt}\"]\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("submitted"), "{err}");
    }

    #[test]
    fn herdr_arg_templates_still_catch_misspelled_placeholders() {
        let err = parse(
            "[harness]\nbackend = \"herdr\"\n\n[harness.herdr]\nact_args = [\"{modle}\"]\n",
        )
        .unwrap_err();
        assert!(err.to_string().contains("not a placeholder"), "{err}");
    }
}
