---
provenance: authored
status: accepted
date: 2026-07-18
---
# 0023 — The founding map leaves `problem/` for a root-level `market.md`

## Context
0013 stratified the model — needs are primitive, the domain is induced
(`need × strategy → domain`) — and filed the invariant layer as
`problem/README.md` (the founding map: needs as `## N-{slug}` anchors,
jobs-to-be-done, market, personas), with the induced subdomains as its siblings
`problem/{subdomain}.md`. That put the induction's *source* and its *result* in
one directory. The tell is in the edges: `strategy/` points into `problem/` in
both directions — `need: problem/README.md#n-{slug}` reaches *backward* into
`problem/` for its source, while `induced-by:` on the subdomains points *forward*
from strategy into `problem/` as the result. A folder that is simultaneously
upstream and downstream of the same neighbor is the structural signature of two
strata still collapsed into one directory. It also strains the founding map's
defining property: the founding map endures pivots, while the rest of `problem/`
is rewritten on every pivot — opposite lifecycles sharing a home.

## Decision
Move the founding map out of `problem/` to a root-level invariant artifact,
`market.md` (title kept: "Founding map"). It holds the demand-side, technology-free
layer: needs (`## N-{slug}` anchors), jobs-to-be-done, market segmentation and
competitive landscape, personas and their as-is journeys. `problem/` then holds
*only* induced subdomains — one role per directory. Strategies ref
`market.md#n-{slug}`. The causal chain reads left-to-right without a folder that is
both source and result: `market.md` (need) → `strategy/` (commit) → `problem/`
(induced) → `solution/` (operate).

## Consequences
Refines 0013's structural binding while leaving its stratification intact: needs
are still primitive, the domain is still induced, and `problem/` still carries
`induced-by:` derivation on every subdomain. `market.md` joins `economics.md` and
`brand.md` as a root-level facet of the business, and its enduring-pivot lifecycle
no longer sits inside the folder pivots rewrite. Mechanical path updates land in
`model.md` (hierarchy, the `need:` schema, rule 11), `rationale.md` (premises 1–2
grounds), `patterns.md` (Personas & market, Pivot), the conventions skill and
lint, the steward mandate/agent derivation-sweep trigger, and the template
(`conventions.md`, `brand.md`, `README.md`, `strategy/first-strategy.md`).

## Alternatives rejected
Needs/market as a prelude inside `strategy/` — the layer that must *endure* pivots
would live in the exact folder pivots *rewrite*; carving "strategy/README endures,
strategy/{x}.md is volatile" gives one folder two opposite lifecycles. A top-level
`needs/` directory-kind — already rejected by 0013 (needs have no independent
lifecycle, failing rule 7's promotion test); this decision keeps needs as
unpromoted anchors in one document and only relocates that document, so the
promotion question stays closed. Leave it in `problem/README.md` — the conflation
this decision exists to remove.
