---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0006 — Recurring content becomes patterns, not structure

## Context
Requirements, personas/market, user journeys, and vendors were candidates for
dedicated structure. Promoting each would grow the tree with groupings rather than
kinds.

## Decision
Promotion test (folded into spec rule 7): a directory is earned only by a new
artifact kind — something needing its own frontmatter, lifecycle, owner, and
provenance distinct from any parent document. Linkability never justifies promotion
(refs target anchors). All four candidates remain body content, described in the
non-normative Patterns & practices section. A proposed `problem/personas/`
directory was explicitly rejected.

## Consequences
The spec gains a three-tier authority gradient: structure (mandatory shape), rules
(mandatory behavior), patterns (advisory practice). Future "does X belong in the
framework?" questions land in patterns by default and promote only via rule 7.
