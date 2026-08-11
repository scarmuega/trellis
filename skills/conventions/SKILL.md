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
conventions.md    schemas, registries, escalation records, boundary guarantees,
                  secrets policy
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
plans/            time-bounded execution: status + type + refs; awaits: edges
                  sequence plans (dispatch holds until targets retire)
decisions/        ADRs, append-only, NNNN-{slug}.md — the why-trail: standing
                  guidance hatches into the governing artifact; a successor's
                  supersedes: names what it replaces (live set = the register)
solution/         context-map.md + {bounded context}/ (README, glossary,
                  contracts/, skills/, deployment units)
org/              {role}/ = mandate.md (always local) + holder/ (agent package:
                  system.md, tools/, skills/, evals/ — or ref.md to external)
archive/          the terminal tier: this same hierarchy at archive/{live path},
                  holding what is finished. Terminal-only; refs still name the
                  live path; never decisions/ (append-only)
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
| Plan counts, throughput, stall/dwell numbers | a generated plan board in `metrics/actuals/` — an instrument with thresholds, never a `definitions.md` metric with a target (Execution health pattern, decision 0030) |
| Time-bounded goal, campaign, experiment, to-be journey | `plans/{plan}.md` with `type:` |
| Big effort needing independently-managed pieces | sibling `plans/{parent}-{piece}.md` files + an umbrella plan whose body indexes them + a registered family tag + `awaits:` edges between pieces where order matters — never a `plans/{plan}/` folder |
| Ordering between plans — advance B only after A finishes | `awaits:` in B's frontmatter — dispatch holds B (still `ready`) until A retires; never umbrella prose alone, never `status: blocked` |
| A blocker, or anything needing a human's decision | an escalation record — fenced `yaml` under `## Escalations` in the artifact it concerns, written by that artifact's owner; never a forge issue, never a separate kind |
| "Why we chose X" | `decisions/NNNN-{slug}.md` — the why stays here; what X obliges goes to the artifact that governs it |
| Standing guidance a decision established | the artifact that governs the behavior — `conventions.md`, a mandate, a BC README or `contracts/`, a `rituals.md` row — citing `(decision NNNN)`; never left only in `decisions/` (Decision hatching pattern) |
| "Which decisions are current?" | `trellis view decisions` or the register in `metrics/actuals/` — a reading over `supersedes:` edges, never a read of all of `decisions/` |
| A language boundary (business or technical function's solution) | `solution/{bc}/` |
| Procedure, playbook, runbook | `solution/{bc}/skills/{skill}/` |
| "Context-free" procedure | it isn't: a convention (`conventions.md`), a ritual (`rituals.md`), a runtime concern, or a missing BC |
| API, IDL, tx3, brand guideline | `solution/{bc}/contracts/` |
| Responsibility + authority (human, agent, or labor vendor) | `org/{role}/mandate.md` |
| Agent prompt, tools, evals | `org/{role}/holder/` |
| Standing process, cadence | `rituals.md` |
| Term of art | `glossary.md` (or BC-local glossary) |
| Vendor providing a system | entry in `solution/context-map.md` with DDD relation |
| An artifact that is finished and will not return | leave it where it is and declare its terminal status (`retired` \| `discarded` \| `archived`); the steward's archive sweep files it to `archive/{its live path}` once it is cold — never move it yourself as part of finishing it |
| "Which plans are live?" / what to pick up next | `trellis plan list` or the generated board in `metrics/actuals/` — a reading over `status:`, never a listing of `plans/` |

Never create: a functions/departments directory, a requirements directory, a
personas directory, a root-level `skills/` directory (skills live in bounded
contexts or agent holder packages — decision 0021), a plan folder
(`plans/{plan}/` — sub-plans are flat siblings, decision 0026), a directory
keyed on a status an artifact can leave (`plans/_drafts/`, `plans/blocked/` —
a path encodes only what is fixed for an artifact's life, spec rule 7), or any
directory that groups existing kinds — groupings are tags; views over tags are
generated. `archive/` is the one lifecycle tier and the exception that shows
the rule: it takes only terminal artifacts, at the one transition that never
reverses, and it mirrors the live hierarchy instead of inventing an axis to
browse by (spec rule 13, decision 0042).

## Rules digest

1. One repo = one root = one domain; the framework's contract ends at the root.
2. Refs (opaque name/URI) are the only boundary-crossing primitive; declare where
   used, never resolve on the framework's behalf.
3. External relationships are knowledge: record them in `context-map.md` with DDD
   relations regardless of physical presence.
4. Mandates are always local; identities are portable, authority is not.
5. State lives elsewhere; never reason from `actuals/` older than the ritual
   interval.
6. Decisions are append-only; supersede, never edit — the successor's
   `supersedes:` names every decision it fully replaces (liveness is computed,
   partial influence stays prose), and a decision is memory, not a manual: its
   standing guidance lands in the governing artifact, citing it.
7. Directories hold kinds; groupings are tags. New directory ⇒ new artifact kind
   (own frontmatter, lifecycle, owner). Linkability alone never justifies one. A
   path encodes only what is fixed for an artifact's life — kind, provenance —
   never a state it can leave; `archive/` (rule 13) is the sole exception.
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
13. Finished artifacts are filed, not deleted: `archive/{live path}` mirrors the
    hierarchy, admits terminal artifacts only, and is never an address — refs
    name the live path on either side of the move. Filing is a separate act
    from finishing, on a declared retention horizon. Archived artifacts stay
    governed (lint, refs, census) and leave attention (dispatch, review gates,
    the effectiveness walk). Never `archive/decisions/`.

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

**Hatch a contract**: when an interface stabilizes — an anchored requirement, a
schema, an API that a second context, a party, or a system now consumes — move
it from prose into the publishing context's `contracts/` as machine-checkable
published language (tx3, IDL, OpenAPI, a brand guideline), leaving the prose as
a pointer. Plans deliver against it (readiness item 11 counts its validation as
verification), the coder lands stabilized interfaces there as part of its PR,
and the steward's derivation sweep escalates contract edits to consumers per
`context-map.md`.

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

**Adopt an implementation role** (a domain whose plans land in code): add a role
per code-bearing bounded context, named for what it does in this domain — mandate
scoping the subdomains and contexts it may change, then `holder/ref.md` pointing
at the plugin's portable `trellis:coder`. Name that role as the `owner:` of
code-bearing plans and dispatch routes to it with no further wiring. The coder
gates on `${CLAUDE_PLUGIN_ROOT}/checks/plan-readiness.md` before starting, blocks
and escalates rather than guessing, delivers code as a PR (draft while
unfinished, never self-merged), and files residue as a `draft` plan for the owner
to release — proposing `complexity:` and `awaits:` on it where it can scope them,
most often awaiting the plan it just advanced. On code-bearing plans the owner
may set `complexity:` at release so
dispatch can size the session — the tier grades reasoning depth, never size, and
absent defers to the binding's default; the mapping to concrete session resources
lives in the instance's runtime binding. Sequencing works the same way: `awaits:`
on a released plan holds its dispatch until every named plan retires. Not a template role: it operates the
business, not the model.

**Raise an escalation**: write a record into the `## Escalations` section of the
artifact that carries the problem — `raised`, `by`, `to` (the raiser's
`escalate-to:`), `status: open`, and the question asked of a human (schema in the
instance's `conventions.md`). Only that artifact's `owner:` writes one, so a role
whose mandate does not reach the artifact reports the finding and the owner
transcribes it — this is why `org/focus` and `org/steward` escalate by reporting.
Resolution is the owner's edit: answer beneath the record, flip
`status: resolved`, and make whatever act it unblocks separately. A `blocked`
plan owes an open record (lint item 24); nothing is deleted.

**Record a decision**: next NNNN, context/decision/consequences/alternatives.
In the same change, land the standing guidance it establishes in the artifact
that governs the behavior, citing `(decision NNNN)` — a decision establishing
nothing standing says `Standing guidance: none.` in its Consequences.
Superseding = new file carrying `supersedes:` with every decision it fully
replaces (partial influence stays prose in its Consequences).

**Compact a decision pile** (a domain whose guidance lives only in the trail):
the owner walks `decisions/` newest→oldest classifying each — still-operative,
superseded-in-prose, or inert. Hatch still-operative guidance into its
governing artifact with a citation; record prose-era supersessions and inert
classifications in `conventions.md`'s decision registry (judgment made once —
the frozen files carry no marks); record the codification as one new decision.
`trellis lint --items 26` names the unreachable set before and after.

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
incomplete pivots, plan refs resolve, plan sequencing edges resolve and stay
acyclic, context-map relations back their language claims, escalation records
well-formed and every blocked plan stating its blocker, mandates have
authority, append-only
decisions, decision supersession well-formed and accepted decisions reachable
from the operative record, registered tags, no secrets, no grouping
directories, actuals freshness).
