---
provenance: authored
status: accepted
date: 2026-07-13
---
# 0022 — The specification is a content-named document set

## Context
`spec/trellis.md` packed three authority tiers into one file: the **grounding**
(the Rationale — premises that describe how the world is, per 0018), the
**normative core** (Hierarchy, Schemas, Rules — the shape and the obligations), and
**non-normative practice** (the Patterns & practices section — adopt, adapt, or
ignore per domain). It was also named after the project: `trellis` is the one
ambient, non-distinctive token in a repo where everything is trellis — exactly what
rule 10 and 0005 (nomenclature) warn against, while the other spec docs are already
content-named (`runtime.md`). 0018 (premises describe, rules oblige) and 0006
(patterns-over-promotion) had already named these as distinct tiers; the single
file kept them entangled, and the Patterns section is the part most prone to growth.

## Decision
Split the specification into one content-named document per authority tier, mirroring
the `runtime.md` companion precedent:

- `spec/rationale.md` — the premises + corollary (grounding).
- `spec/model.md` — Hierarchy, Schemas, Rules, plus the front-door intro and a doc
  map; renamed from `trellis.md`. This is the version anchor and entry point.
- `spec/patterns.md` — the non-normative Patterns & practices.
- `spec/runtime.md` — unchanged; the existing non-normative operation companion.

Nothing normative changes — only filing, splitting, and a rename — so the spec stays
v11 (filing-only, per 0018). Rule references inside the moved patterns read "spec
rule N" for cross-document clarity, matching `runtime.md`.

## Consequences
Each file carries a single authority tier, and its name says which. `model.md` is
the front door and the version anchor; instances still adopt "the spec" through
`decisions/0000-adopt-trellis.md` unchanged (it pins a version, not a path). Live
references (README, `runtime.md`, the conventions skill) point at the new names; two
historical references keep the old `trellis.md` path by the append-only rule
(0009's Decision text and the initial-prototype CHANGELOG entry) — they record the
state at their time.

## Alternatives rejected
Keep everything inline — mixes three tiers in one file and lets the growth-prone
patterns section bloat the normative core. Fold patterns into `runtime.md` — wrong
axis; runtime is execution/binding, patterns are content placement. Keep the
`trellis.md` name — ambient token, against the framework's own nomenclature rule.
Leave the Rationale in `model.md` — keeps grounding and obligations entangled, the
very separation 0018 drew.
