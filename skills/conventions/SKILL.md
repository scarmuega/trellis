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
problem/          founding map (README: needs as ## N-{slug} anchors, market,
                  personas — technology-free, endures pivots) + {subdomain}.md
                  carrying induced-by: strategy edges, class on the edge
strategy/         {strategy}.md — committed solution to a need: status (only
                  committed induces), need ref, differentiation, core-ranking
brand.md          the promise to customers
economics.md      pricing, revenue model, unit economics
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
| Market need, job-to-be-done | anchored `## N-{slug}` section in `problem/README.md` — technology-free |
| Market segmentation, competitive landscape | `problem/README.md` body |
| Persona, as-is journey | anchored section in `problem/README.md` (persona names → glossary) |
| Business model, chosen means, differentiation, pivot | `strategy/{strategy}.md` — only `committed` induces |
| New problem area | `problem/{subdomain}.md` with `induced-by:` edges (class on the edge) |
| Requirement / constraint | body of its subdomain or BC README, anchored `## R-{name}-{n}` |
| Promise, positioning, voice | `brand.md` |
| Pricing, revenue, unit economics, sizing | `economics.md` |
| Metric definition or target | `metrics/definitions.md` |
| Metric values | `metrics/actuals/` (generated) or state-ref — never authored |
| Time-bounded goal, campaign, experiment, to-be journey | `plans/{plan}.md` with `type:` |
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
contexts or agent holder packages — decision 0021), or any directory that
groups existing kinds — groupings are tags; views over tags are generated.

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
    the edge; only committed strategies induce; retiring a strategy orphans its
    subdomains (re-parent or archive).

## Procedures

**Scaffold a domain**: copy the plugin's `${CLAUDE_PLUGIN_ROOT}/template/`; fill `problem/README`,
`brand.md`, `economics.md` first; replace `<owner>` placeholders; adjust
`conventions.md` registries and boundary guarantees; date and pin
`decisions/0000-adopt-trellis.md`; delete `template/README.md`.

**Commit a strategy**: create `strategy/{name}.md` — `need:` pointing at a
founding-map `## N-{slug}` anchor, `differentiation:`, `status: aspirational`
until it is the model as operated, then `committed` (only committed induces).
More than one core edge under it ⇒ record `core-ranking:` (scarcest attention
first).

**Add a subdomain**: create `problem/{name}.md`, declare its `induced-by:` edges —
each naming the strategy that put it in the domain, with `class:` on the edge.
Effective automation policy is the strictest class across committed edges
(generic ⇒ agent autonomy; supporting ⇒ agent executes, human samples; core ⇒
human review gate; orphans default to core). Add its terms to the glossary.

**Pivot**: set the old strategy `status: retired`; commit the new one with
`supersedes:`. Every subdomain the retired strategy induced is orphaned — the
founding map endures; re-parent each orphan onto a surviving commitment
(re-classifying its edge) or archive it. Orphans carry core policy until
resolved.

**Add a bounded context**: `solution/{name}/` with README + local glossary; add a
row to `context-map.md`; put its interfaces in `contracts/`, procedures in
`skills/`.

**Create a plan**: `/trellis:plan {topic}` where the binding offers it — the
harness's plan mode gathers research and approval, then the artifact persists to
`plans/{slug}.md` with a registered `type:`, resolving refs, and `status: draft`.
Without the command: draft manually per the instance's plan schema, registering
new types and tags in `conventions.md` first.

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
edges, strategy validity, orphan detection, core-ranking, technology-free
founding map, incomplete pivots, plan refs resolve, mandates have authority,
append-only decisions, registered tags, no secrets, no grouping directories,
actuals freshness).
