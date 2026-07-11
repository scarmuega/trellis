---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0013 — Stratified model: needs are primitive, the domain is induced

## Context
Spec v10's premise 1 made problem and solution "the two first-class spaces" and
treated `problem/` as primitive. Working the model against pivot scenarios showed
the problem space conflates two strata: market needs (external, slow-changing,
technology-free — invariant across any pivot) and the operational problem space a
business must master *because of how* it chose to fulfill them. Aircraft
maintenance is in an airline's domain only because the strategy put it there.
Defining the domain by the current bet ("we are an AI agents company") is the
symptom of collapsing the strata.

## Decision
Adopt the stratified model. Needs are primitive — the only invariant layer.
Strategy is a committed solution to a need: only commitments induce consequences;
intentions do not. The domain is induced, not found: `need × strategy → domain`.
Problem and solution are roles, not territories — a solution at level N is the
problem context at level N+1. Structurally: `problem/README.md` becomes the
founding map (needs as anchored `## N-{slug}` sections, jobs-to-be-done, market,
personas — technology-free, enduring pivots), and derived artifacts carry their
derivation: every subdomain must be attributable to the strategy commitment that
induced it (new spec rule 11).

## Consequences
Partially supersedes 0007: the premise enumeration is replaced (the rationale is
rewritten to ten premises), while the grounded-premise + deletion-corollary
mechanism stands. Reconciles 0005: the names `problem`/`solution` stand, now read
as stratum-role labels rather than fixed territories. Pivot acquires defined
semantics: strategy edits are the normal unit of pivot; needs edits are rare and
cascade everywhere.

## Alternatives rejected
A top-level `needs/` kind — needs have no lifecycle independent of the founding
map, failing rule 7's promotion test (the same reasoning that kept personas
unpromoted in 0006). Keeping the problem space primitive with strategy as an
annotation — hides the derivation, so untagged subdomains stay plausible and
zombie subdomains survive pivots.
