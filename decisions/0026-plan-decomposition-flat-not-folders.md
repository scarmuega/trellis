---
provenance: authored
status: accepted
date: 2026-07-21
---
# 0026 — Plan decomposition: flat siblings, never plan folders

## Context
Can a plan be a directory — `plans/{plan}/` with an index and sub-plans managed
independently? The answer was derivable but unstated: the hierarchy defines a
plan as `plans/{plan}.md`; rule 7 earns a directory only for a new artifact
kind, and a sub-plan is the same kind; the Stratified alternation pattern
already says recursion happens via fractal roots, never deeper trees. Derivable
is not stated (0024) — the question recurs at every instance that outgrows
single-file plans, and the capability it reaches for (pieces of a program
tackled and retired independently) is legitimate.

## Decision
Codify the flat shape as a `spec/patterns.md` pattern (Plan decomposition):
each piece a sibling `plans/{parent}-{piece}.md` with full plan frontmatter and
an independent lifecycle; the umbrella a plan like any other whose body carries
the decomposition as an anchored index; family membership a registered tag,
roll-ups generated over it; prefix naming for adjacency (rule 10). Name
`plans/{plan}/` as a lint item 11 violation by name, as 0021 did for
root-level `skills/`. When a family accretes its own goals, metric definitions,
and plans about its plans, rule 8 fires: it becomes a domain root.

## Consequences
Non-normative filing only — the spec stays v11 and no schema changes.
"Can plans nest?" has a stated answer for operators (the pattern) and an
enforcement point for the steward (lint 11). Independent management — the need
nesting reaches for — is served by what plans already have: per-file status,
owner, and refs.

## Alternatives rejected
Allowing `plans/{plan}/` folders — breaks rule 7 (same kind, so a grouping);
the index file would need either no frontmatter or a second plan schema; a path
would no longer determine its artifact's schema, and every walk over `plans/`
grows recursive special cases. A `parent:` frontmatter field — a normative
schema change (spec bump) for a relation body links and tags already express,
and nothing in the lint or effectiveness walks would consume it. A new
`program` kind — still time-bounded execution carrying status and type; the
open plan-type registry already names it without new structure.
