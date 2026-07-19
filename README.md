# Trellis

> *A domain-driven operating model for businesses run by humans and AI agents.*

Trellis structures a business as a versioned markdown filesystem — one repo per
domain — that humans and AI agents operate through the same artifacts: a founding
map of market needs, committed strategies and the problem space they induce,
solution contexts, mandates with explicit authority, plans with lifecycle,
decisions as append-only memory, and rituals as the heartbeat.

**Status: pre-1.0.** Specification v11, zero production usage-hours. Expect churn;
conventions harden through dogfooding, and every change lands in `decisions/`.

## Layout

This repo is a **Claude Code plugin** named `trellis`. Claude auto-discovers
skills, agents, commands, and hooks from the conventional directories below.

| path | what |
|---|---|
| `.claude-plugin/plugin.json` | plugin manifest (name, description, author) |
| `.claude-plugin/marketplace.json` | self-distributing catalog: the `trellis` plugin under the `scarmuega` marketplace |
| `spec/model.md` | the specification (normative core + front door): what Trellis is, the doc map, hierarchy, schemas, rules |
| `spec/rationale.md` | rationale: the premises the framework rests on — grounds the rules in `spec/model.md` |
| `spec/patterns.md` | patterns & practices companion (non-normative): recurring content inside the structure — business functions, requirements, procedures, automation shapes, personas, journeys, vendors, pivot, stratified alternation, portfolio inversion |
| `spec/runtime.md` | runtime companion (non-normative): trigger planes, the `act` primitive, contract services, the Claude Code + forge reference binding |
| `template/` | copy to scaffold a new domain root (steward and focus roles included) |
| `checks/conventions-lint.md` | canonical lint checklist, shared by the steward agent and the conventions skill |
| `checks/plan-effectiveness.md` | canonical plan-effectiveness checklist, shared by the focus agent and `/trellis:focus` |
| `evals/` | shared eval harness (runner, grader, fixture skeleton, clean-root precision guard) + per-member suites; `evals/focus/` grades the effectiveness prompt against seeded fixture domains |
| `skills/` | skills, one dir each (`SKILL.md` + assets) |
| `skills/conventions/` | the base skill (`trellis:conventions`): conventions, placement guide, procedures |
| `agents/` | subagents, one `{name}.md` each |
| `agents/steward.md` | `trellis:steward` — enforcement agent; the portable holder of every domain's steward role |
| `agents/focus.md` | `trellis:focus` — plan-effectiveness agent; the portable holder of every domain's focus role |
| `commands/` | slash commands: `/trellis:act` (role-invocation primitive), `/trellis:ritual` (ritual runner), `/trellis:plan` (plan authoring through harness plan mode), `/trellis:focus` (plan-effectiveness review) |
| `hooks/hooks.json` | deterministic enforcement gate: append-only decisions, no hand-edits to `generated` artifacts |
| `CHANGELOG.md` | the release ledger; notable changes per version |
| `decisions/` | ADRs governing this spec — the framework eats its own mechanics |

`spec/`, `template/`, and `checks/` sit at the plugin root: they are shared family
artifacts that every skill and agent draws on, not the property of any one skill.

## Quick start

1. Read `spec/model.md` — the front door: it opens with the what and the why, maps
   the doc set, and holds the Rules. The premises live in `spec/rationale.md`.
2. Scaffold: `cp -r template/ path/to/{your-domain}` and follow the copied
   `README.md`.
3. Install into Claude Code: `/plugin marketplace add scarmuega/trellis`, then
   `/plugin install trellis@scarmuega`. The skill, spec, template, and steward agent
   ship together.
4. The template ships with `org/steward/` and `org/focus/` — mandates stay
   local; each holder is a `ref.md` to the plugin's agent (identity is portable,
   authority is not). Deterministic tooling (a CLI) gets extracted from the steward
   later, once usage shows which checks must never be probabilistic.
5. Operate: work interactively at the domain root; invoke roles with
   `/trellis:act`, rituals with `/trellis:ritual`, draft plans with
   `/trellis:plan`, and challenge them with `/trellis:focus`; wire the
   scheduled and event-driven planes with the template's `.github/workflows/`.
   The runtime contract and its reference binding: `spec/runtime.md`.

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
- **Slash commands** → `commands/{name}.md`. Bare verb: `act`, `ritual` →
  `/trellis:act`, `/trellis:ritual`. **Hooks** → `hooks/hooks.json`.

The bare-name rule doubles as the selection rule: a verb a person invokes
deliberately is a command; a topic that should load on recognition is a skill;
a role that runs to completion under bounded tools is an agent; a guard
needing no judgment is a hook. When shapes compete, decision 0020 is the
worked example of choosing; the harness-neutral rubric is the Automation
shapes pattern (`spec/patterns.md`).

## Non-goals

Cross-root composition — how domains share, import, publish, or aggregate — is
out of scope by design (spec rule 1). Use your tooling of choice; state your
guarantees in your instance's `conventions.md`.
