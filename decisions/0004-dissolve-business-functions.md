---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0004 — No functions branch: business functions dissolve into the ontology

## Context
The original draft had a `mktg` branch, later generalized to `funcs/{function}`.
This introduced a second decomposition axis (traditional org chart) competing with
the problem-space decomposition, giving artifacts two plausible homes. A function
with its own goals, metrics, and plans is a shadow domain.

## Decision
Business functions are not a branch. The function's problem is a subdomain
(classified core/supporting/generic per business), its solution is a bounded
context, its campaigns are plans, its standing processes are rituals, its ownership
is a mandate. Codified as spec rules 7 (kinds vs tags) and 8 (shadow domains) and
the Business functions pattern.

## Consequences
The automation policy applies to business functions. "Everything mktg" becomes a
generated view over tags. `solution/` is relabeled solution space — business and
technical alike — since under agents every function is operationalized in software.

## Alternatives rejected
Functions as a bag of procedures (splits the skills mechanism); functions as
planning units with own KPIs (shadow domains — if real, they are domains, rule 8).
