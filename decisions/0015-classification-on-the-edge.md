---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0015 — Classification lives on the (subdomain, strategy) edge; effective policy is strictest-wins

## Context
v10 stored `class: core|supporting|generic` as an intrinsic property of the
subdomain node. Under the stratified model (0013) classification is relative to
strategy: core is where the strategy claims differentiation; supporting is what it
drags in but doesn't compete on; generic is where the induced problems coincide
with everyone else's, so the market already solved them. The same problem
classifies oppositely under different strategies — payments is generic for almost
everyone and core for Stripe. A node-level `class:` cannot express that.

## Decision
Subdomain frontmatter replaces the node-level `class:` with `induced-by:` — a list
of edges, each naming an inducing `strategy/{strategy}.md` and carrying `class:`
for that pair. The effective automation policy is the strictest class across edges
to *committed* strategies (core > supporting > generic); a subdomain with no
committed edge is an orphan and defaults to core until resolved. Edges to
`aspirational` or `retired` strategies contribute nothing — only commitments
induce.

## Consequences
Refines 0004: business functions are "classified per strategy edge," not "per
business." The automation policy — the framework's lever for agent autonomy —
survives the move and stays computable by a stateless agent from a single file
with no arbitration. Strictest-wins because the policy is a risk control on
differentiation claims, and risk composes by max, not average. The scheme fails
safe: a modeling error produces a human review gate, never accidental autonomy.
More than one core edge under a single strategy triggers the `core-ranking:`
requirement on the strategy node.

## Alternatives rejected
Class on the node with a strategy annotation — loses the multi-edge case (one
subdomain shared across ventures or strategies). Average or most-recent-wins
policy derivation — dilutes the risk control precisely where strategies disagree.
A separate edges file — splits one subdomain's identity across artifacts.
