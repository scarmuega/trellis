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

#[derive(Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct RuntimeConfig {
    pub server: Server,
    pub scheduler: Scheduler,
    pub harness: Harness,
    /// The errand table (decision 0051): name → prompt template. See
    /// `default_prompts` — the three shipped entries merge under whatever
    /// the instance declares, and any extra key is a new requestable errand.
    pub prompts: std::collections::HashMap<String, String>,
    pub sessions: Sessions,
    /// Push transports for newly opened escalation records. The seam is
    /// 0038's; the one adapter that ships is `kind = "herdr"` (decision
    /// 0043). Any other kind is still refused rather than quietly ignored.
    pub channels: Vec<ChannelCfg>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        RuntimeConfig {
            server: Server::default(),
            scheduler: Scheduler::default(),
            harness: Harness::default(),
            prompts: default_prompts(),
            sessions: Sessions::default(),
            channels: Vec::new(),
        }
    }
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
    /// without leaving a verdict, and was relinquished to `ready`. The daily
    /// cadence used to be this backstop by accident; a tight loop needs it on
    /// purpose, or a crash-looping act respawns every tick.
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

/// What a finished herdr session's workspace does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Retain {
    /// Keep the scene of an unaccounted end — a recycled or lost session —
    /// for attach; close the ones whose plan says what happened.
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
    /// Close their workspaces.
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
    /// Seconds a settled session is granted before the runtime concludes its
    /// plan was abandoned and hands it back (decision 0061).
    ///
    /// This is the one clock the disk-verdict rule needs, and it is bounding
    /// a *stall*, not work: a session with something to say says it in the
    /// plan — a verdict, or a declared `handoff:` for work that outlives the
    /// turn — and either answer retires the pane immediately, whatever this
    /// is set to. What the grace buys is the benefit of the doubt for a
    /// session between turns.
    pub idle_grace_secs: u64,
}

/// Below this, the grace is short enough that a slow turn boundary could be
/// read as a stall. Noted at startup rather than refused: a tight loop is a
/// legitimate thing to want, and guessing at what the operator meant is the
/// habit decision 0061 exists to break.
pub const SHORT_GRACE_SECS: u64 = 60;

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
            idle_grace_secs: 120,
        }
    }
}

/// How sessions are run. There is one way — herdr panes (decision 0061) —
/// so this is the herdr table plus the keys that used to choose, kept only
/// long enough to tell an instance what became of them.
#[derive(Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Harness {
    /// The agent flags, socket, and session policy. Herdr owns the
    /// executable (`kind`), and the prompt is submitted to the running TUI
    /// rather than carried in argv.
    pub herdr: HerdrHarness,
    /// Removed with the process backend (decision 0061). Captured rather
    /// than rejected as an unknown field so `validate` can say where the
    /// setting went instead of where it was.
    pub backend: Option<String>,
    pub act_cmd: Option<Vec<String>>,
    pub ritual_cmd: Option<Vec<String>>,
}

/// The bridge every default template ends on, ahead of `{procedure}`.
const PROCEDURE_BRIDGE: &str = "The procedure below is the trellis framework's \
     commands/act.md, rendered into this prompt so no installed plugin is \
     required; the domain root's trellis.toml and domain.md are authoritative over it where \
     they differ.";

/// The trigger table: the two prompts the runtime fires on its own — `act`
/// per dispatchable plan, `ritual` per due row — and nothing else. `act` is
/// still the only procedure the binary carries, and `{procedure}` always
/// renders its body (decision 0051, narrowed by 0060).
///
/// The operator's errand is deliberately *not* here. It was a third entry
/// (`refine`) in a map any instance could extend with named errands of its
/// own, and 0060 removed the whole notion: an errand is an ask written at
/// the moment it is wanted, so its content is the operator's instruction and
/// its framing is `ERRAND_PROMPT`, which no config can name or override.
///
/// Every default opens with a line naming what the session is for, which is
/// what an operator sees at the top of the pane when they attach.
pub fn default_prompts() -> std::collections::HashMap<String, String> {
    let mut map = std::collections::HashMap::new();
    map.insert(
        "act".to_string(),
        format!(
            "{{plan}} — dispatched act as {{owner}} (trellis runtime).\n\
             \n\
             Act as {{owner}}: advance {{plan}} toward its objective — make the next \
             increment of progress within your authority. The runtime has \
             pre-verified the mechanical readiness items and this plan's holds, the \
             acting-role marker is already stamped (leave it alone), the plan is \
             already claimed for you — it went ready → active as this session started, \
             so never claim it again — and the change \
             mechanics are computed: {{automation}}; core never lands, it is proposed. \
             Evaluate only the judgment items. Your mandate and any local holder \
             package are rendered below, so the procedure's read-the-mandate and \
             adopt-the-holder steps are already done — a holder ref naming a plugin \
             agent still adopts inline per the procedure. \
             On an uncleared blocker `trellis plan block {{plan}} --by \
             {{owner}} --asks …` (escalations go to {{escalate_to}}); write escalation \
             records with `trellis escalate add`; leave a trail. End with a verdict, \
             spelled as a command: retire it (`trellis plan retire {{plan}}`); block \
             it (`trellis plan block {{plan}} --by {{owner}} --asks …`); where your \
             mandate names a hand-off, pass it to the mandated next taker (`trellis \
             plan pass {{plan}} --to <role>` — the mandate's spelling wins for the \
             role); or — if you leave a proposal for its owner to rule on and no \
             hand-off is mandated — park it on that proposal (`trellis plan handoff \
             {{plan}} <pr>`). The runtime reads completion from the plan itself, \
             not from your session: a plan left active with no handoff is returned \
             to ready once you go idle, and dispatched again. That includes work \
             you park in the background — a long build, a monitor, anything that \
             outlives the turn: declare it with `trellis plan handoff` or it is \
             lost with the session.\n\
             \n\
             {{mandate}}\n\
             \n\
             {{holder}}\n\
             \n\
             {{skills}}\n\
             \n\
             {PROCEDURE_BRIDGE}\n\
             \n\
             {{procedure}}"
        ),
    );
    map.insert(
        "ritual".to_string(),
        format!(
            "ritual {{ritual}} — executed by {{executor}} (trellis runtime).\n\
             \n\
             Execute the ritual \"{{ritual}}\" from rituals.md as {{executor}}: read \
             its row and take the procedure (a skill ref or inline steps) and the \
             cadence — the freshness window for any metrics involved. The runtime \
             resolved the executor and stamped the acting-role marker (leave it \
             alone); escalations go to {{escalate_to}}. The executor's mandate and \
             any local holder package are rendered below, so the procedure's \
             read-the-mandate and adopt-the-holder steps are already done. Deliver \
             findings through the \
             instance's escalation channel; an executor that owns no artifact reports \
             each finding verbatim, addressed to the owner who transcribes it.\n\
             \n\
             {{mandate}}\n\
             \n\
             {{holder}}\n\
             \n\
             {{skills}}\n\
             \n\
             {PROCEDURE_BRIDGE}\n\
             \n\
             {{procedure}}"
        ),
    );
    map
}

/// The operator's errand, framed (decision 0060). One prompt, no name, not
/// in `[prompts]` and not overridable — the errand's *content* is the
/// instruction the operator wrote, and this supplies only what the
/// instruction cannot: who to act as, that the runtime already resolved and
/// stamped, where escalations go, whether the ask is delegated execution
/// (0057), the role context, the skills index, and the act procedure.
///
/// It states no contract of its own. `refine` used to carry one — write only
/// the plan artifact, never claim it, never touch the solution — which was
/// right for refinement and wrong as a default for an ask nobody had written
/// yet. What bounds a session here is what bounds every session: the
/// mandate's `scope:` and `authority:`, and the artifact's automation class.
pub fn errand_prompt() -> String {
    format!(
        "errand on {{plan}} as {{owner}} (trellis runtime).\n\
         \n\
         Act as {{owner}} over {{plan}}. The operator's instruction, verbatim, is \
         the whole of the ask — do that and nothing beyond it:\n\
         \n\
         {{instruction}}\n\
         \n\
         {{delegation}}\n\
         \n\
         The runtime has already resolved you as the plan's owner and verified the \
         plan is live and owned; the acting-role marker is already stamped (leave \
         it alone); your mandate and any local holder package are rendered below, \
         so the procedure's read-the-mandate and adopt-the-holder steps are \
         already done; escalations go to {{escalate_to}}. This is an operator's ask, \
         not a dispatch: the plan's status is not yours to flip unless the \
         instruction says so, and it was not claimed for you. Stay inside the \
         mandate's scope: and authority:; the touched artifact's automation class \
         still decides how the change lands, and core never lands, it is proposed. \
         Anything the ask requires beyond that authority is an escalation, not an \
         improvisation. Report what changed and what you escalated.\n\
         \n\
         {{mandate}}\n\
         \n\
         {{holder}}\n\
         \n\
         {{skills}}\n\
         \n\
         {PROCEDURE_BRIDGE}\n\
         \n\
         {{procedure}}"
    )
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
        let mut cfg: RuntimeConfig =
            basic_toml::from_str(&text).map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        // The shipped errands merge under the instance's [prompts]: an entry
        // overrides its name, an absent name keeps the default, and any extra
        // name is a new errand (decision 0051).
        for (name, template) in default_prompts() {
            cfg.prompts.entry(name).or_insert(template);
        }
        cfg.validate()
            .map_err(|e| anyhow::anyhow!("{}: {e}", path.display()))?;
        Ok(cfg)
    }

    /// Everything worth refusing before the first tick rather than at the
    /// first spawn.
    pub fn validate(&self) -> anyhow::Result<()> {
        self.validate_retired_keys()?;
        for (name, template) in &self.prompts {
            tmpl::check(&format!("prompts.{name}"), std::slice::from_ref(template))?;
        }
        // Framework-authored and unreachable from config, so a failure here
        // is this binary's bug rather than the instance's — checked all the
        // same, because the placeholder set is what both share.
        tmpl::check("the errand prompt", std::slice::from_ref(&errand_prompt()))?;
        if self.scheduler.tick_secs == 0 {
            anyhow::bail!("scheduler.tick_secs must be at least 1");
        }
        if self.scheduler.max_concurrent == 0 {
            anyhow::bail!("scheduler.max_concurrent must be at least 1 — 0 spawns nothing");
        }
        self.validate_herdr()?;
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

    /// What a config that still chooses a backend is told. A refusal rather
    /// than a shrug: these keys used to decide how every session ran, and an
    /// instance that set one deliberately deserves to hear that the decision
    /// is gone rather than watch it be ignored.
    fn validate_retired_keys(&self) -> anyhow::Result<()> {
        if let Some(backend) = &self.harness.backend {
            if backend != "herdr" {
                anyhow::bail!(
                    "harness.backend = \"{backend}\" — the process backend was removed \
                     (decision 0061): sessions run as interactive agents in herdr panes, and \
                     completion is read from the plan on disk rather than from an exit code. \
                     Drop the key and run a herdr server, or pin the previous release"
                );
            }
        }
        for (name, set) in [
            ("act_cmd", self.harness.act_cmd.is_some()),
            ("ritual_cmd", self.harness.ritual_cmd.is_some()),
        ] {
            if set {
                anyhow::bail!(
                    "harness.{name} was removed with the process backend (decision 0061) — \
                     herdr owns the executable (harness.herdr.kind) and the prompt is \
                     submitted to the running agent, so what is left to configure is flags: \
                     move them to harness.herdr.{}",
                    if name == "act_cmd" {
                        "act_args"
                    } else {
                        "ritual_args"
                    }
                );
            }
        }
        Ok(())
    }

    /// What the harness refuses before the first tick. The themes: the
    /// prompt is not an argument, and a budget would be a lie.
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
        assert_eq!(cfg.harness.herdr.act_args[0], "--permission-mode");
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
    fn the_trigger_prompts_merge_under_instance_overrides() {
        let cfg = parse("[prompts]\nritual = \"custom {ritual}\"\n").unwrap();
        assert_eq!(cfg.prompts["ritual"], "custom {ritual}");
        // The name the instance did not touch keeps its default.
        assert!(cfg.prompts["act"].contains("{procedure}"));
    }

    #[test]
    fn the_table_carries_the_triggers_and_nothing_else() {
        // The operator's errand is not a `[prompts]` entry and has no name to
        // declare (decision 0060) — an instance adding a key gets a prompt
        // nothing fires, which `errand_prompt` exists so nobody needs.
        let prompts = default_prompts();
        let mut names: Vec<&String> = prompts.keys().collect();
        names.sort();
        assert_eq!(names, vec!["act", "ritual"]);
    }

    #[test]
    fn the_errand_prompt_frames_what_the_instruction_cannot() {
        // The ask is the operator's; the framing is the runtime's, and this
        // is the whole of it (decision 0060).
        let errand = errand_prompt();
        for key in [
            "{instruction}",
            "{delegation}",
            "{escalate_to}",
            "{mandate}",
            "{holder}",
            "{skills}",
            "{procedure}",
        ] {
            assert!(errand.contains(key), "the errand prompt lost {key}");
        }
        // Placeholder discipline is shared with the configurable templates.
        tmpl::check("errand", std::slice::from_ref(&errand)).unwrap();
        // It states no write contract of its own: what bounds the session is
        // the mandate and the automation class, not a clause copied from
        // refine, whose contract fit refinement and nothing else.
        assert!(!errand.contains("never claim the plan"));
    }

    #[test]
    fn the_act_verdict_clause_spells_every_exit_as_a_command() {
        // The transcript audit's lesson (decision 0059): a directive embedded
        // as a concrete command is followed; one described as an outcome is
        // improvised.
        let act = &default_prompts()["act"];
        assert!(act.contains("`trellis plan retire {plan}`"));
        assert!(act.contains("`trellis plan pass {plan} --to <role>`"));
        assert!(act.contains("`trellis plan handoff {plan} <pr>`"));
        assert!(act.contains("`trellis plan block {plan} --by {owner} --asks …`"));
    }

    #[test]
    fn a_prompt_template_still_catches_misspelled_placeholders() {
        let err = parse("[prompts]\nritual = \"{instrucion}\"\n").unwrap_err();
        assert!(err.to_string().contains("instrucion"), "{err}");
    }

    #[test]
    fn a_misspelled_placeholder_is_caught_at_load() {
        let err = parse("[harness.herdr]\nact_args = [\"--model\", \"{modl}\"]\n").unwrap_err();
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
    fn the_harness_defaults_to_herdr_with_its_knobs() {
        let cfg = parse("").unwrap();
        assert_eq!(cfg.harness.herdr.kind, "claude");
        assert_eq!(cfg.harness.herdr.retain, Retain::OnFailure);
        assert_eq!(cfg.harness.herdr.on_shutdown, OnShutdown::Detach);
        assert!(!cfg.harness.herdr.act_args.is_empty());
    }

    #[test]
    fn the_herdr_knobs_parse() {
        let cfg = parse(
            "[harness.herdr]\nretain = \"always\"\non_shutdown = \"stop\"\nidle_grace_secs = 30\n",
        )
        .unwrap();
        assert_eq!(cfg.harness.herdr.retain, Retain::Always);
        assert_eq!(cfg.harness.herdr.on_shutdown, OnShutdown::Stop);
        assert_eq!(cfg.harness.herdr.idle_grace_secs, 30);
    }

    /// The keys that used to choose a backend are refused by name rather
    /// than ignored: an instance that set one meant it.
    #[test]
    fn the_process_backend_is_refused_with_somewhere_to_go() {
        let err = parse("[harness]\nbackend = \"process\"\n").unwrap_err();
        assert!(err.to_string().contains("0061"), "{err}");
        assert!(err.to_string().contains("herdr"), "{err}");

        // …and naming the one that remains is merely redundant.
        assert!(parse("[harness]\nbackend = \"herdr\"\n").is_ok());
    }

    #[test]
    fn a_process_argv_is_refused_and_points_at_the_flags_table() {
        for key in ["act_cmd", "ritual_cmd"] {
            let err = parse(&format!("[harness]\n{key} = [\"claude\", \"-p\"]\n")).unwrap_err();
            assert!(err.to_string().contains("0061"), "{err}");
            assert!(err.to_string().contains("harness.herdr."), "{err}");
        }
    }

    #[test]
    fn a_budget_under_the_herdr_backend_is_refused_as_unenforceable() {
        let err = parse("[harness.herdr]\nact_args = [\"--max-budget-usd\", \"{budget}\"]\n")
            .unwrap_err();
        assert!(err.to_string().contains("--print"), "{err}");
        assert!(err.to_string().contains("0043"), "{err}");
    }

    #[test]
    fn print_mode_under_the_herdr_backend_is_refused() {
        let err = parse("[harness.herdr]\nact_args = [\"-p\"]\n").unwrap_err();
        assert!(err.to_string().contains("interactive"), "{err}");
    }

    #[test]
    fn a_prompt_placeholder_under_the_herdr_backend_is_refused() {
        let err = parse("[harness.herdr]\nritual_args = [\"{prompt}\"]\n").unwrap_err();
        assert!(err.to_string().contains("submitted"), "{err}");
    }

    #[test]
    fn herdr_arg_templates_still_catch_misspelled_placeholders() {
        let err = parse("[harness.herdr]\nact_args = [\"{modle}\"]\n").unwrap_err();
        assert!(err.to_string().contains("not a placeholder"), "{err}");
    }
}
