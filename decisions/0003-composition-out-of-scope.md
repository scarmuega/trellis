---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0003 — Cross-root composition is out of scope; refs are the only primitive

## Context
An intermediate draft defined `deps/` (materialized imports), `pub/` (export
surface), and a declaration manifest. These are dependency-management plumbing, and
mature tooling for that problem already exists (submodules, symlinks, package
managers, vendoring).

## Decision
The framework's contract ends at the root. The only boundary-crossing primitive is
the ref: an opaque name/URI inside an artifact. Refs are knowledge ("this relates to
that"); resolution is the consumer's tooling of choice.

## Consequences
The spec stays a knowledge ontology rather than a package manager. Relationship
knowledge survives in `solution/context-map.md` (DDD relation types). Guarantees the
plumbing used to provide — read-only imports, version pinning, reproducibility of
agent context — must be restated in each instance's `conventions.md`.

## Alternatives rejected
Framework-defined `deps/` + `pub/` + manifest (v3): correct engineering, wrong layer
for a methodology spec.
