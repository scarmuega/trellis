---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0009 — Distribute Trellis as a Claude Code plugin

## Context
Trellis needs a distribution mechanism so humans and agents receive the skill, the
spec, and the scaffold from one install. The first working shape bundled the spec
and template *inside* the skill directory so the skill was self-contained when
consumed remotely. Once the direction clarified toward multiple granular skills and
subagents, a single self-contained skill stopped being the right unit of
distribution.

## Decision
The repo is a Claude Code plugin named `trellis` (`.claude-plugin/plugin.json`).
Capabilities are auto-discovered from the convention directories — `skills/`,
`agents/`, and later `commands/`, `hooks/` — so adding one needs no manifest edit.
Shared artifacts live at the **plugin root**: `spec/trellis.md` (the specification)
and `template/` (the domain scaffold) are family assets that every skill and agent
draws on, not the property of any one skill. Skills reach them via
`${CLAUDE_PLUGIN_ROOT}`. `agents/` ships empty, ready for the first subagent.

## Consequences
The unit of consumption is the plugin, not any individual skill — the base skill is
intentionally **not** standalone-installable, because it depends on plugin-root
spec/template. `decisions/` and `README.md` ship as governance material but are not
runtime-discovered. Extraction of deterministic tooling (a CLI) from the steward
role, noted in the quick start, stays out of scope here.

## Alternatives rejected
Bundle spec/template inside each skill for standalone installability — the session's
initial position, never committed. Rejected once multiple skills and agents became
the direction: it forces either duplication across skills or dangling cross-skill
references. A plugin-root shared home avoids both and matches the vercel-plugin
layout.
