---
name: conventions
description: Operating conventions for Trellis domain repos — a domain-driven structure for businesses run by humans and AI agents. Use when working inside a Trellis root (markers at root: conventions.md, problem/, solution/, org/), when scaffolding a new domain, deciding where business content belongs, editing artifacts with Trellis frontmatter, adding subdomains/bounded-contexts/roles, recording decisions, or linting a domain for convention compliance.
---

# Trellis

> The structure your business grows along. A domain-driven operating model for
> businesses run by humans and AI agents.

One repo = one root = one domain. Everything under the root is the domain's own
knowledge, state-refs, and org. Full spec: `${CLAUDE_PLUGIN_ROOT}/spec/model.md`
(plugin root), or the ref in the instance's `decisions/0000-adopt-trellis.md`.

## Detecting a Trellis root

A directory whose root holds `conventions.md`, `problem/`, `solution/`, and `org/`.
Always read the instance's `conventions.md` before writing anything — it is
authoritative over this skill where they differ.

## The map

```
conventions.md    schemas, registries, boundary guarantees, secrets policy
rituals.md        heartbeat: cadenced processes, executor roles, escalations
glossary.md       ubiquitous language — cite terms from here
market.md         founding map: the invariant layer — needs as ## N-{slug}
                  anchors, market, personas — technology-free, endures pivots
problem/          induced subdomains: {subdomain}.md carrying induced-by:
                  strategy edges, class on the edge
strategy/         {strategy}.md — committed solution to a need: status (the
                  maturity ladder; the committed band induces), need ref,
                  differentiation, funded-by edges (what sustains it),
                  core-ranking
brand.md          the promise to customers
economics.md      pricing, revenue model, unit economics — narrates the
                  funded-by skeleton in strategy/
metrics/          definitions.md (authored) + actuals/ (generated or state-refs)
plans/            time-bounded execution: status + type + refs
decisions/        ADRs, append-only, NNNN-{slug}.md
solution/         context-map.md + {bounded context}/ (README, glossary,
                  contracts/, skills/, deployment units)
org/              {role}/ = mandate.md (always local) + holder/ (agent package:
                  system.md, tools/, skills/, evals/ — or ref.md to external)
```

## Placement guide — where does this content go?

| Content | Destination |
|---|---|
| Market need, job-to-be-done | anchored `## N-{slug}` section in `market.md` — technology-free |
| Market segmentation, competitive landscape | `market.md` body |
| Persona, as-is journey | anchored section in `market.md` (persona names → glossary) |
| Business model, chosen means, differentiation, pivot | `strategy/{strategy}.md` — only `committed` induces |
| New problem area | `problem/{subdomain}.md` with `induced-by:` edges (class on the edge) |
| Requirement / constraint | body of its subdomain or BC README, anchored `## R-{name}-{n}` |
| Promise, positioning, voice | `brand.md` |
| Pricing, revenue, unit economics, sizing | `economics.md` |
| What sustains a strategy — self-capture, a funder, a conversion thesis | `funded-by:` edges in `strategy/{strategy}.md` frontmatter; the narrative stays in `economics.md` |
| Metric definition or target | `metrics/definitions.md` |
| Metric values | `metrics/actuals/` (generated) or state-ref — never authored |
| Time-bounded goal, campaign, experiment, to-be journey | `plans/{plan}.md` with `type:` |
| Big effort needing independently-managed pieces | sibling `plans/{parent}-{piece}.md` files + an umbrella plan whose body indexes them + a registered family tag — never a `plans/{plan}/` folder |
| "Why we chose X" | `decisions/NNNN-{slug}.md` |
| A language boundary (business or technical function's solution) | `solution/{bc}/` |
| Procedure, playbook, runbook | `solution/{bc}/skills/{skill}/` |
| "Context-free" procedure | it isn't: a convention (`conventions.md`), a ritual (`rituals.md`), a runtime concern, or a missing BC |
| API, IDL, tx3, brand guideline | `solution/{bc}/contracts/` |
| Responsibility + authority (human, agent, or labor vendor) | `org/{role}/mandate.md` |
| Agent prompt, tools, evals | `org/{role}/holder/` |
| Standing process, cadence | `rituals.md` |
| Term of art | `glossary.md` (or BC-local glossary) |
| Vendor providing a system | entry in `solution/context-map.md` with DDD relation |

Never create: a functions/departments directory, a requirements directory, a
personas directory, a root-level `skills/` directory (skills live in bounded
contexts or agent holder packages — decision 0021), a plan folder
(`plans/{plan}/` — sub-plans are flat siblings, decision 0026), or any
directory that groups existing kinds — groupings are tags; views over tags are
generated.

## Rules digest

1. One repo = one root = one domain; the framework's contract ends at the root.
2. Refs (opaque name/URI) are the only boundary-crossing primitive; declare where
   used, never resolve on the framework's behalf.
3. External relationships are knowledge: record them in `context-map.md` with DDD
   relations regardless of physical presence.
4. Mandates are always local; identities are portable, authority is not.
5. State lives elsewhere; never reason from `actuals/` older than the ritual
   interval.
6. Decisions are append-only; supersede, never edit.
7. Directories hold kinds; groupings are tags. New directory ⇒ new artifact kind
   (own frontmatter, lifecycle, owner). Linkability alone never justifies one.
8. A unit needing its own goals, metrics, and plans is a domain — a new root, not
   a subdirectory.
9. The framework is fractal: portfolios and holdcos are just domains.
10. Names are retrieval keys: distinctive over ambient, no opaque acronyms.
11. The domain map carries its derivation: every subdomain declares `induced-by:`
    strategy edges — an untagged subdomain is a modeling error; class lives on
    the edge; only committed strategies induce (the band
    `validated|implemented|established` of the maturity ladder); discarding a
    strategy orphans its subdomains (re-parent or archive).
12. A strategy declares what sustains it: committed strategies carry
    `funded-by:` edges — `self`, another strategy, or an external ref, with
    `relation: intended` marking a conversion thesis (documents, never
    sustains). A committed strategy with no sustaining edge is an economic
    orphan; discarding a strategy orphans its funding dependents (re-fund,
    convert, or reconsider).

## Procedures

**Scaffold a domain**: copy the plugin's `${CLAUDE_PLUGIN_ROOT}/template/`; fill `market.md`,
`brand.md`, `economics.md` first; replace `<owner>` placeholders; adjust
`conventions.md` registries and boundary guarantees; date and pin
`decisions/0000-adopt-trellis.md`; delete `template/README.md`.

**Advance a strategy**: create `strategy/{name}.md` at `status: raw` — `need:`
pointing at a founding-map `## N-{slug}` anchor, `differentiation:`. Advance on
evidence, never ambition (Strategy maturity pattern): `defined` when need and
differentiation are falsifiable, `validated` when the experiment's decision
criterion resolves — the commitment line, induction starts — `implemented` when
operating, `established` when the work is optimization; `discarded` is
terminal. More than one core edge under it ⇒ record `core-ranking:` (scarcest
attention first). At `validated`, the funding declaration is owed: `funded-by:`
edges naming what sustains it — `self` if it captures its own revenue, else
the capture strategy's ref (`relation: intended` for a not-yet-operating
conversion thesis).

**Add a subdomain**: create `problem/{name}.md`, declare its `induced-by:` edges —
each naming the strategy that put it in the domain, with `class:` on the edge.
Effective automation policy is the strictest class across committed edges
(generic ⇒ agent autonomy; supporting ⇒ agent executes, human samples; core ⇒
human review gate; orphans default to core). Add its terms to the glossary.

**Pivot**: set the old strategy `status: discarded`; commit the new one with
`supersedes:`. Every subdomain the discarded strategy induced is orphaned — the
founding map endures; re-parent each orphan onto a surviving commitment
(re-classifying its edge) or archive it. Orphans carry core policy until
resolved. Strategies the discarded one funded are economic orphans the same
way: re-fund each onto a surviving capture, commit its conversion thesis, or
reconsider it.

**Add a bounded context**: `solution/{name}/` with README + local glossary; add a
row to `context-map.md`; put its interfaces in `contracts/`, procedures in
`skills/`.

**Create a plan**: `/trellis:plan {topic}` where the binding offers it — the
harness's plan mode gathers research and approval, then the artifact persists to
`plans/{slug}.md` with a registered `type:`, resolving refs, and `status: draft`.
Without the command: draft manually per the instance's plan schema, registering
new types and tags in `conventions.md` first.

**Challenge the plans**: `/trellis:focus [scope]` where the binding offers it —
evaluates `ready`, `active`, and `blocked` plans against the problem they answer to,
metrics as evidence (canonical checklist:
`${CLAUDE_PLUGIN_ROOT}/checks/plan-effectiveness.md`). Findings are advisory:
accepted candidates graduate through `/trellis:plan`; everything else escalates
to its owner. The scheduled twin is the `focus` ritual in `rituals.md`.

**Add a role**: write `mandate.md` FIRST — purpose, scope, explicit `authority:`
(spend, publish, approve) and `escalate-to:` — then the holder. Agent holders get
`system.md` + at least one eval. Humans and external vendors get `holder/ref.md`.

**Record a decision**: next NNNN, context/decision/consequences/alternatives.
Superseding = new file that names the one it replaces.

**Before creating any new directory**: apply rule 7's promotion test. Default
answer is body content with a stable anchor.

**Respect provenance**: check `owner:` before editing anything; never hand-edit
`generated`; anything you generate carries `provenance: generated`.

## Lint checklist

For audits, run the canonical checklist at
`${CLAUDE_PLUGIN_ROOT}/checks/conventions-lint.md` — the same set the
`trellis:steward` agent enforces (frontmatter validity, subdomain derivation
edges, strategy validity, orphan detection, funding-edge validity, economic
orphans, capture points, core-ranking, technology-free founding map,
incomplete pivots, plan refs resolve, mandates have authority, append-only
decisions, registered tags, no secrets, no grouping directories, actuals
freshness).
