---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0016 — Derivation upkeep: provenance lint, orphan collection, derivation sweep

## Context
A derived domain map drifts. Strategies retire, needs get edited, and subdomains
outlive the commitment that induced them. The failure mode is the zombie
subdomain: an organizational capability maintained out of inertia, justified by a
strategy nobody operates anymore.

## Decision
Three mechanisms, all escalation-only. (1) Lint: rule 3 rewritten for `induced-by:`
edges, plus new rules 13–17 — strategy-file validity, orphan detection,
core-ranking presence, a technology-free founding map, and an incomplete-pivot
check (a root whose strategies are all retired is operating nothing it can
attribute). (2) A new weekly
steward ritual, the **derivation sweep**: when `problem/README.md` or `strategy/`
changed since the last sweep, escalate every downstream artifact — induced
subdomains, `context-map.md`, plans referencing affected subdomains, mandates
scoped to them — to its owner for revalidation, and flag newly orphaned subdomains
for re-parenting or archival. (3) The dependency chain is normative (spec rule
11): needs → strategy → domain map → classification → bounded contexts and org.

## Consequences
The steward's hard boundary is unchanged: it never deletes or edits authored
content. Garbage collection is an escalation — the owner does the collecting. Lint
rules 1–12 keep their numbers (rule 6 is cited by 0011); new rules append. Orphans
carry core policy (0015) until resolved, so drift tightens control rather than
loosening it.

## Alternatives rejected
Steward auto-deletion of orphans — violates the flag-only mandate, and a pivot's
fallout deserves owner judgment. Cascade revalidation as a lint rule — lint is
static pass/fail over the tree, while the cascade is edit-triggered staleness: a
ritual concern, not a checklist item.
