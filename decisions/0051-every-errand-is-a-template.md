---
provenance: authored
status: accepted
date: 2026-08-12
---
# 0051 — Every errand is a template; the only command is act

## Context

0050 made the spawn prompt self-contained, but the binary still knew three
errands by name: `Prompts { act, ritual, refine }` as struct fields,
`Procedures { act, refine, ritual }` with a composition hardcoded in Rust,
a refine-only request queue, and a refine-only route. Santiago's observation
cut through it: refine and ritual are not primitives — 0048 already said
`act` is the sole role-invocation primitive and everything else composes it —
so nothing about them belongs in the binary beyond a name looked up in a
table. The runtime's own model agrees: rituals have always been
config-defined rows in `rituals.md`; the errand definitions were the odd
ones out.

## Decision

**`[prompts]` is a name → template map, and `act` is the only procedure the
binary carries.** Three framework-authored entries ship as defaults — `act`
(what the dispatch loop fires), `ritual` (what the cadence pass fires),
`refine` (the shipped operator-requestable errand) — merged under whatever
the instance declares, each name overridable independently. Any *additional*
key is a new operator-requestable errand with no recompile: `[prompts] audit
= """…"""` and the board offers it. `{procedure}` always renders the act
body, embedded from `commands/act.md` alone (`--plugin-root` still swaps a
live checkout's copy); what an errand adds on top — the refinement contract,
the ritual framing, whatever an instance invents — lives in its template.

This narrows two standing sentences honestly. 0048's "the contract lives at
the framework level, not in a runtime prompt string" becomes: the contract's
*default* is framework-authored — it ships in the binary's default template
and in `template/runtime.toml`, both upstream artifacts — and an instance
that rewrites its template owns the variant, exactly as it owns a rewritten
`rituals.md` row or mandate. `commands/refine.md` and `commands/ritual.md`
are no longer embedded; they remain the interactive plane's slash commands.
0050's mechanism survives with a smaller footprint: one embedded file, no
composition.

**The request plane generalizes with it.** The refine queue becomes an
errand queue (`ErrandRequest { errand, plan, instruction }`, latest ask per
plan wins); the route becomes `POST /api/plans/{slug}/errands/{name}` with
`…/refine` kept as the alias it shipped under; `GET /api/errands` lists the
requestable names so the board's menu is the config's, not the client's.
`act` and `ritual` are reserved for their triggers and refused as requests —
the loop and the cadence pass are plumbing, and plumbing is what remains
hardcoded. Validation, the in-flight refusal, the nudge, the shared `plan:`
keyspace, and serve's verbatim relay are unchanged; every requested errand
spawns through `act_cmd` under the plan owner's mandate, so a new template
grants nothing a refine did not — the free-text instruction was already the
prompt door the loopback default guards.

## Consequences

- `trellis dispatch request <errand> <plan> "<instruction>"` is the CLI
  spelling; `trellis dispatch refine` stays as the alias.
- A misspelled `[prompts]` key is no longer refused — it is a new errand
  nothing triggers. The price of an open table; `GET /api/errands` makes it
  visible.
- An instance's old flat `[prompts]` overrides keep working unchanged: the
  keys were already the table's names.
- spec/runtime.md, template/runtime.toml, and the server module doc amended
  in the same commit (the 0041 precedent).
