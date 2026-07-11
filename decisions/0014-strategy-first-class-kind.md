---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0014 — Strategy is a first-class kind at `strategy/`

## Context
v10 had no strategy artifact. The business model as operated was implicit across
`brand.md` (positioning), `decisions/` ("why we chose X"), and `plans/`
(time-bounded execution). The stratified model (0013) makes strategy the inducing
act — it cannot stay implicit, because subdomain provenance edges must point at
something with a lifecycle.

## Decision
New top-level kind: `strategy/{strategy}.md`, between `problem/` and `brand.md`.
Frontmatter: `status: aspirational | committed | retired` (only `committed`
induces), `need:` (ref to a founding-map `## N-{slug}` anchor), `differentiation:`
(why we win, against which alternative), `core-ranking:` (required iff the
strategy has more than one core edge — a total order, scarcest attention first),
optional `supersedes:` (set on pivot). Strategy passes rule 7's promotion test: it
has its own lifecycle, its own owner, and frontmatter no parent document carries.
`brand.md` is reframed as the customer-facing projection of the committed
strategy's differentiation claim. Multiple strategy files may coexist (shared
substrate, staged pivots).

## Consequences
The top level still reads as a sentence (0005): problem, strategy, brand,
economics, metrics, plans, decisions, solution, org. Pivots become diff-able:
retire one file, commit another, `supersedes:` records the lineage. The
root-detection markers (`conventions.md`, `problem/`, `solution/`, `org/`) are
unchanged — `strategy/` is not required for detection.

## Alternatives rejected
A plan type — strategies are not time-bounded; they persist while committed. A
decision genre — decisions are append-only and immutable, while a live strategy is
edited (its core-ranking changes). Inside `problem/` — strategy is the *solution*
at market level. Inside `solution/` — strategy is the *problem-generator* at
operations level; bounded contexts are downstream of the subdomains it induces.
