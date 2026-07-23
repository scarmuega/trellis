---
provenance: authored
status: accepted
date: 2026-07-23
---
# 0028 — Strategies declare economic lineage: `funded-by:` edges

## Context
Strategies decompose by need and differentiation (0013/0014), not by revenue
stream — so in many real portfolios some strategies produce value (adoption,
installed base, catalog content; open-source devtools are the canonical case)
while others capture it (grants, hosting, paid trust services). The model
cannot express that one strategy's sustainability depends on another's: a
value-producing strategy looks economically self-justifying, the focus ritual
(0025) has no basis to challenge its spend, and the portfolio-level failure
mode — the funding strategy dies before the conversion strategies reach the
committed band — is invisible to every ritual. Rationale premise 1 already
states the missing knowledge ("the search must sustain itself economically to
continue — viability is knowledge in its own right"), but its only ground was
`economics.md` prose, which is unlintable. Meanwhile 0015 established the
precedent this gap wants: the edge is the primitive, and semantics ride on the
edge.

## Decision
A new frontmatter field on `strategy/{strategy}.md` and spec rule 12: every
committed-band strategy declares `funded-by:` — a list of edges naming where
the value it produces is captured. The edge's `strategy:` slot takes `self`
(the strategy captures its own revenue), a `strategy/{funder}.md` ref, or an
opaque external ref (rule 2, following the mandate `holder:` precedent — a
domain sustained by its portfolio says so). Semantics ride the edge:
`relation: current` (the default) is the operating model as committed;
`relation: intended` is a conversion thesis — a strategy funded today by
grants but meant to convert into a hosting business carries both edges. The
edge declares designed value flow only: how real the operation is stays on the
maturity ladder (0027), how capture performs stays in metrics, and
`economics.md` remains the narrative and pricing home over the machine-readable
skeleton — no amounts or split ratios on edges. A value-capture strategy is a
genuine strategy whose `need:` is the payer's need (canonical explainer: a new
Economic lineage pattern in `spec/patterns.md`). The lint gains items 19–21:
edge validity; the economic orphan — a committed strategy with no `current`
edge to `self`, a committed-band strategy, or an external ref, same spirit and
severity as an orphaned subdomain (edges to aspirational strategies and all
`intended` edges document, never sustain); and the capture point — a committed
band funded only by itself in a circle captures nothing. The derivation sweep
flags the funding dependents of a discarded strategy exactly as it flags
induced subdomains (Pivot pattern extended), and plan-effectiveness gains an
appended Economic lineage group (items 16–17 — funder health challenges the
producer's spend; a stalled conversion thesis on a running subsidy; items 1–15
stay byte-identical for the eval contracts). The spec bumps to v13; instances
re-pin per lint 18 — migration is one `funded-by:` block per committed
strategy, `self` for strategies that capture their own revenue. Surfaces
updated: model schema and hierarchy, rationale premise 1's grounds, both
checklists, the conventions skill (map, placement row, rules digest,
procedures), the steward's derivation sweep, both focus surfaces' evidence
base, `/trellis:plan` research, the template (conventions schema, strategy
stub, economics.md, rituals row, README, v13 pin), and the eval fixtures
(self-funded).

## Consequences
Sustainability becomes a lintable property of the strategy graph instead of
prose: the open-source portfolio is expressible (producers name their grants
strategy `current` and their hosting strategy `intended`), the focus ritual can
challenge attention on producers against the health of whatever captures their
value, and a pivot's economic fallout is collected like its derivational
fallout. A committed strategy subsidized indefinitely with no capture design
now carries a standing finding — that is signal, not noise: the honest record
of a viability debt. Economic lineage stays strategy-layer knowledge; the
founding map never carries it (0023's boundary holds). Follow-ups flagged: a
funding-lineage defect fixture and contract in `evals/focus/` (a producer
funded by a declining grants strategy), and a steward eval suite exercising
lint items 19–21.

## Alternatives rejected
Leaving sustainability in `economics.md` prose — the status quo; unlintable,
and the focus ritual has no basis to challenge a producer's spend. A scalar
node field (`revenue: self | {ref}`) — loses the multi-funder case and the
current-plus-intended conversion thesis; 0015 already taught that semantics
belong on the edge. A separate revenue-stream kind — a revenue stream that
answers a payer's need *is* a strategy; a new directory would be a grouping of
an existing kind (rule 7). Folding funding into the maturity ladder (a
`subsidized` stage) — conflates how real the operation is with who pays for
it; unlike 0027's two-axis rejection, these axes need no cross-field
constraints, which is the test that they are genuinely two. Amounts or
percentages on edges — false precision; split ratios are `economics.md`
narrative and metric definitions. Transitive solvency as the normative check —
the one-hop sustaining check plus the capture-point item catches the failures
that matter; deeper accounting is the focus walk's judgment, not a pass/fail
lint.
