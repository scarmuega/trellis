---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0011 — The steward's holder is the plugin's `trellis:steward` agent

## Context
The template shipped the steward as a full `org/steward/` role: a local `mandate.md`
plus an embedded `holder/` package (`system.md` + `evals/conventions-lint.md`). The
holder is generic — the system prompt and the 12-item lint checklist are identical
for every domain — while only the mandate is domain-specific. With Trellis now
packaged as a plugin (0009), carrying a copy of the generic holder in every
scaffolded domain guarantees drift and forgoes any central improvement path. The
checklist was also near-duplicated in the `trellis:conventions` skill's lint section.

## Decision
Split the steward along the seam rule 4 already defines — *mandates are always
local; identities are portable, authority is not*:

- The **holder** becomes the plugin agent `agents/steward.md` (`trellis:steward`),
  the portable identity. Its prompt binds to whatever domain root invoked it and
  reads that root's `org/steward/mandate.md` for authority at run time.
- The **mandate** stays local: the template keeps `org/steward/mandate.md`.
- The template's `holder/` becomes a `ref.md` → `trellis:steward` — the first
  exercise of the spec's "holder as ref to external" form.
- The lint checklist is lifted to one canonical plugin asset,
  `checks/conventions-lint.md`, referenced by both `trellis:steward` and the
  `trellis:conventions` skill (previously near-duplicates).

## Consequences
Scaffolded domains carry only their mandate plus a one-line ref; the steward
implementation lives and improves in one place, and the CLI-extraction charter now
has a single target. A domain's steward requires the `trellis` plugin installed —
consistent with 0009. A domain needing a bespoke steward still can: replace the ref
with a `system.md` package, per checklist rule 6 (package holders need an eval; ref
holders are exempt). Trellis dogfoods its own portable-holder path for the first time.

## Alternatives rejected
Keep the whole role in the template — drift-prone, no central improvement,
duplicates the skill's lint digest. Make the steward a pure plugin agent with no
template presence — violates rule 4: it strips the local mandate that scopes the
steward's authority per domain and breaks the invariant that every `org/{role}` =
mandate + holder.
