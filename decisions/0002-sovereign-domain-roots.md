---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0002 — One repo = one root = one domain

## Context
Early drafts modeled multiple domains inside one monorepo. This forced a growing set
of accommodations — a `shared/` carve-out, promote-don't-copy rules, a matrix-vs-
nested orgchart debate — all symptoms of forcing a graph into a single tree.

## Decision
Each domain is a sovereign root: one repo, one root, one domain. The tree inside a
root stays pure; cross-domain relationships move to the boundary.

## Consequences
The repo boundary becomes the security and permission boundary for agents (physical,
not conventional). Domain lifecycle (spin-out, sale, shutdown, archive) is trivial.
Portfolio-level views become aggregations performed by a holdco domain rather than
free greps. The framework is fractal: a portfolio is just another domain.

## Alternatives rejected
Multi-domain monorepo with `shared/` (boundaries enforced only by convention, which
agents can violate); matrix orgchart with root-level people refs (domains stop being
self-contained).
