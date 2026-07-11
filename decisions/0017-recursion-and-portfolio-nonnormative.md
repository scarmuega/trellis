---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0017 — Recursion and portfolio checks stay non-normative

## Context
The stratified model recurs in principle at every level: a bounded context's
chosen runtime induces sub-problems exactly as the market strategy induces
subdomains. And across ventures sharing a substrate, one venture's generic
subdomain being another's core marks a factoring boundary. Should the machinery —
provenance edges, lint, garbage collection — be mandatory at every stratum and
across roots?

## Decision
Normative machinery stops at the top stratum (needs → strategy → subdomains).
Below it, the **Stratified alternation** pattern applies conceptually: a bounded
context's significant commitments are `decisions/` whose Consequences name what
they induce; superseding one orphans those consequences. When a lower stratum
genuinely needs its own goals, metrics, and plans, rule 8 fires and it becomes a
new domain root — where the full machinery applies again. Recursion happens via
fractal roots (rule 9), not deeper trees. The **Classification inversion
(portfolio)** pattern runs at a portfolio root over its venture refs; no
cross-root mechanics are defined (0003 stands).

## Consequences
The anti-ceremony premise holds: full per-level provenance frontmatter at every
depth would be the documentation ceremony the framework exists to eliminate. The
three-tier authority gradient (0006) absorbs both concerns — structure and rules
at the top stratum, patterns below and across.

## Alternatives rejected
Full per-level frontmatter recursion — ceremony explosion, and premise-level
contradiction. A normative cross-root inversion check — composition plumbing,
wrong layer for a methodology spec per 0003.
