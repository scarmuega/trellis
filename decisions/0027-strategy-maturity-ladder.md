---
provenance: authored
status: accepted
date: 2026-07-21
---
# 0027 — Strategy status becomes the maturity ladder

## Context
The three-value strategy enum (`aspirational | committed | retired`, 0013/0014)
records the commitment but not the position in the search that rationale
premise 1 says the business is. Focus (0025) evaluates every committed strategy
uniformly: a strategy mid-validation and a long-established one get the same
coverage expectation and the same metric read, and the checklist cannot say the
one thing a young strategy most needs — your next plan is an experiment, not
another initiative. Aspirational strategies are invisible to the walk entirely,
so a validation debt never surfaces. And retirement conflates two endings that
deserve different readings: superseded by a pivot versus killed by the
evidence. A two-field design — keep `status:`, add an "orthogonal" `maturity:`
— was drafted and rejected: it needed cross-field constraints (`implemented`
requires `committed`; `discarded` requires `retired`), and axes that need
constraints between them are one axis at two resolutions. Every old status
value is exactly a band of maturity stages; the seemingly informative
off-diagonal cell, committed-but-unvalidated, dissolves on inspection —
operating a strategy is itself the strongest validation experiment, so that
strategy is simply `implemented` and its stage-work (monitoring) is what
surfaces failure.

## Decision
Replace the enum:
`status: raw | defined | validated | implemented | established | discarded`.
Each stage names the work owed — refine, validate, implement, monitor, harden —
and the metrics that read it; the canonical stage→work table is a new
`spec/patterns.md` pattern (Strategy maturity). The coarse vocabulary survives
as bands defined in the schema: committed = `validated|implemented|established`
(the commitment line sits at `validated`, whose stage-work — implementation —
is when induced subdomains must materialize), aspirational = `raw|defined`,
retired = `discarded`. Rule 11's phrase "only committed strategies induce"
stays true verbatim, and every prose use of "committed strategy" across the
spec, checks, and surfaces keeps its meaning without edits. `discarded` is the
sole terminal — failed, unmeritorious, or superseded; the successor's
`supersedes:` ref distinguishes replaced from killed. The spec bumps to v12;
lint item 18 walks pinned instances through re-pinning with this migration
mapping: aspirational → `raw` or `defined`; committed → `validated`,
`implemented`, or `established` as the evidence supports; retired →
`discarded`. Consumers updated: plan-effectiveness gains an appended Maturity
group (items 14–15 — stage-work planned, metrics match the stage; items 1–13
stay byte-identical for the eval contracts), lint items 13/14/17 reword their
literal enum matches, both focus surfaces widen the evidence base past the
committed band (`raw` exempt), `/trellis:plan` step 2 proposes the
stage-matched plan type, and the template and eval fixtures migrate.

## Consequences
Focus gains something to say about pre-committed strategies — a `defined`
strategy with no experiment is a finding, not invisible — and about strategies
past their prime: metric relevance shifts from learning indicators through
delivery and outcomes to unit economics as the stage advances. The two
retirement endings separate cleanly. Existing instances migrate deliberately on
re-pin (lint 18), one line per strategy. Decisions 0013/0014 remain unedited as
the record of the superseded semantics (rule 6). Follow-ups flagged: a
`stage-mismatch` defect fixture and contract in `evals/focus/`, and varying
fixture stages to exercise the metric-stage match across bands.

## Alternatives rejected
A second `maturity:` field alongside `status:` — the constraint smell above;
two fields for one concept doubles the edit surface and demands a
disambiguation rule between them. A seventh `superseded` value — derivable from
the successor's `supersedes:` ref; a value the graph already states is
redundant vocabulary. A non-normative pattern-only label — the enum carries
rule 11's inducing gate, and an optional lifecycle cannot gate induction.
Inferring maturity from artifacts present — inference cannot distinguish
`discarded` from superseded, and young instances lack the trail.
