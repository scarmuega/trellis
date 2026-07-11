# Trellis

> **The structure your business grows along.**
> *A domain-driven operating model for businesses run by humans and AI agents.*

Trellis structures a business as a versioned markdown filesystem — one repo per
domain — that humans and AI agents operate through the same artifacts: problem and
solution spaces, mandates with explicit authority, plans with lifecycle, decisions
as append-only memory, and rituals as the heartbeat.

**Status: pre-1.0.** Specification v10, zero production usage-hours. Expect churn;
conventions harden through dogfooding, and every change lands in `decisions/`.

## Layout

This repo is a **Claude Code plugin** named `trellis`. Claude auto-discovers
skills, agents, commands, and hooks from the conventional directories below.

| path | what |
|---|---|
| `.claude-plugin/plugin.json` | plugin manifest (name, description, author) |
| `.claude-plugin/marketplace.json` | self-distributing catalog: the `trellis` plugin under the `scarmuega` marketplace |
| `spec/trellis.md` | the specification: rationale, hierarchy, schemas, rules, patterns |
| `template/` | copy to scaffold a new domain root (steward role included) |
| `checks/conventions-lint.md` | canonical lint checklist, shared by the steward agent and the conventions skill |
| `skills/` | skills, one dir each (`SKILL.md` + assets) |
| `skills/conventions/` | the base skill (`trellis:conventions`): conventions, placement guide, procedures |
| `agents/` | subagents, one `{name}.md` each |
| `agents/steward.md` | `trellis:steward` — enforcement agent; the portable holder of every domain's steward role |
| `CHANGELOG.md` | the release ledger; notable changes per version |
| `decisions/` | ADRs governing this spec — the framework eats its own mechanics |

`spec/`, `template/`, and `checks/` sit at the plugin root: they are shared family
artifacts that every skill and agent draws on, not the property of any one skill.

## Quick start

1. Read `spec/trellis.md` — the Rationale section is the why, the rest is the what.
2. Scaffold: `cp -r template/ path/to/{your-domain}` and follow the copied
   `README.md`.
3. Install into Claude Code: `/plugin marketplace add scarmuega/trellis`, then
   `/plugin install trellis@scarmuega`. The skill, spec, template, and steward agent
   ship together.
4. The template ships with `org/steward/` — the mandate stays local; its holder is
   a `ref.md` to the plugin's `trellis:steward` agent (identity is portable,
   authority is not). Deterministic tooling (a CLI) gets extracted from the steward
   later, once usage shows which checks must never be probabilistic.

## Extending the plugin

Follow Claude Code's directory conventions — auto-discovery, no manifest edits
needed. The plugin supplies the namespace: a skill or agent named `foo` is
invoked as `trellis:foo`, so name members by their **bare topic** and never
repeat the `trellis` prefix.

- **Skills** → `skills/{name}/SKILL.md`. Bare topic: `conventions`, `scaffold`,
  `lint` → `trellis:conventions`, `trellis:scaffold`, `trellis:lint`.
- **Subagents** → `agents/{name}.md`: YAML frontmatter (`name`, `description`,
  optional `tools`, `model`) then the system prompt as the body. Bare role:
  `steward` → `trellis:steward`.
- **Slash commands** → `commands/{name}.md` (add the dir when the first one
  lands). **Hooks** → `hooks/hooks.json`.

## Non-goals

Cross-root composition — how domains share, import, publish, or aggregate — is
out of scope by design (spec rule 1). Use your tooling of choice; state your
guarantees in your instance's `conventions.md`.
