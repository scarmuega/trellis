---
provenance: authored
owner: scarmuega
status: draft
type: initiative
complexity: standard
---
# `trellis metrics diff` — the metric sweep's mechanical share

## Context

The steward's metric sweep is fully manual: parse targets out of
`metrics/definitions.md`, parse values out of `metrics/actuals/`, join the two
over each plan's `metrics:` refs, respect the freshness window. Only the
staleness half exists in code (lint item 12). An opus session re-derives the
join every week.

## Objective

`trellis metrics diff [--format json]`: one row per (plan, metric) —
`target`, `actual`, `deviation`, `fresh: bool`, `owner` — so the sweep session
judges which deviations matter instead of computing them.

## Approach

- **Blocked on a format decision first**: `metrics/actuals/` today has no
  parseable shape guaranteed by convention — values may be tables, prose, or
  state-refs. The plan's first deliverable is a decision (`decisions/NNNN`)
  fixing a minimal machine-readable convention for actuals (e.g. a table with
  `metric | value | date` columns, or front-matter key/values), additive so
  existing prose actuals degrade to `actual: unparseable` rows rather than
  errors.
- Targets: `metrics/definitions.md` anchors already carry them; reuse the
  anchor scan `markdown.rs` provides and lint item 12's freshness machinery.
- The join is the `metrics:` ref list dispatch/readiness already extract.
- `agents/steward.md`'s metric-sweep section then shrinks to: run the diff,
  judge the rows, report per owner.

## Measurement

The metric-sweep ritual session's transcript contains no by-hand parsing of
`actuals/` — its reads are the diff output plus whatever a judgment call needs.

## Risks and escalation triggers

If the actuals-format decision can't stay additive (existing domains would
have to rewrite their actuals), stop and escalate — a convention change that
breaks standing domains is the owner's call, not this plan's.
