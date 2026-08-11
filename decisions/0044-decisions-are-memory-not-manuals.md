---
provenance: authored
status: accepted
date: 2026-08-08
---
# 0044 — Decisions are memory, not manuals

## Context
`decisions/` is the one kind with no exit. Rule 6 freezes an accepted decision
on commit, rule 13 bars the tier from taking it — deliberately, since the
archive's address stability exists precisely so append-only decisions never
rot into dangling refs — and 0042 already noted the pressure valve the kind
does have: `NNNN-` self-sorts by age, so the glob-and-scroll problem plans had
is mild here. What decisions lack is different. Supersession is prose-only —
0013 partially supersedes 0007 in a Consequences sentence, 0018 supersedes
0013's enumeration in another — so liveness is uncomputable: a reader crosses
the whole pile to learn which guidance still operates. 0027 made a strategy's
`superseded` state derivable from the successor's `supersedes:` ref; decisions
had no such ref to derive from. And nothing says where operative guidance must
live, so a domain can accumulate standing behavior that exists nowhere but the
trail — a pile of deltas a reader must mentally merge. 0042's hook — reconsider
if the tier itself grows into the problem it solved — is this decision's
trigger, and the tier can never be its answer.

## Decision
A decision is memory, not a manual, and the model now says so twice (spec
v17 → v18).

**Guidance hatches.** The standing guidance an accepted decision establishes
lands, in the same change, in the artifact that governs the behavior — the
instance's `conventions.md`, a mandate, a context's README or `contracts/`, a
`rituals.md` row, `glossary.md` — citing the decision, so operating never
requires reading the trail (rule 6 amendment; Decision hatching pattern). A
decision establishing nothing standing declares `Standing guidance: none.` in
its own Consequences. The compact operational artifact is therefore the
*editable, owned* one, which consolidates under its owner's pen with git as
its trail — decisions stay the why.

**Supersession is structural.** Decisions gain their first schema block —
codifying the `status: accepted` the gate and lint item 9 already keyed on —
with an optional `supersedes:` list on the successor naming every decision it
fully replaces. A decision is live iff no accepted decision supersedes it;
partial influence stays prose. The successor carries the edge, so the frozen
target is never touched and the gate needs no carve-out. `trellis view
decisions` renders the register (live, superseded-by, reachable-via) at
`metrics/actuals/decision-register.md`.

**The kernel computes reachability, the owner judges it.** Lint item 26:
supersession edges resolve (never self, never a non-decision, globs match
exactly one, no cycles), path-style decision refs in authored bodies resolve,
and every accepted, live decision is reachable — cited by a live authored
artifact outside `decisions/`, declared inert, or listed in the new decision
registry in `conventions.md`, where prose-era dispositions land once instead
of being re-derived every sweep. The unreachable set is a judgment note, never
a violation: adoption must not flood a domain with findings over its own
history. The derivation sweep escalates members to their owner as orphaned
operative content; a domain arriving with a pile runs the skill's
"Compact a decision pile" procedure once.

## Consequences
Spec v18: the rule 6 amendment, the decision schema block, a rule 13 tail, the
Decision hatching pattern. The kernel gains `supersedes:` derivation with
liveness and cycle detection, lint item 26, the `decisions` view, and the
decision-registry read; the template gains the schema block, a `## Decisions`
section, the registry, and the derivation-sweep clause; re-pins ride the bump
per lockstep. CODEOWNERS still lists every decision — the review gate matters
*more* for a frozen file, and generated lines cost no attention.

This repo adopts prospectively: no backfill of 0001–0043. The mechanism is for
implementing domains — this plugin root is not a Trellis root, no lint runs
here, and its trail-to-spec propagation already operates as the "Changing the
model" procedure plus the lockstep tests. Decisions recorded here from now on
cite forward per the pattern.

## Alternatives rejected
**Consolidation by restatement** — a periodic decision superseding N old ones
and restating their guidance inside `decisions/`. It freezes on acceptance
like any other, so its first correction forces another wholesale copy —
append-only digests re-accumulate — and it plants operative guidance in the
one kind nobody should have to read, competing with the artifacts that govern.
`supersedes:` stays what it is everywhere else: the mark of a choice genuinely
replaced.

**Archiving decisions** — forecloses on rules 6 and 13 together; the tier is
address-stable *because* decisions are its long-lived readers.

**A generated digest alone** — a reading answers *what is live?* but cannot
author coherence (0042's line), and a view that resummarized authored judgment
would break the verbatim-relay rule; guidance must be owned.

**`supersedes:` frontmatter alone** — dead decisions leave the live set, but
current guidance still requires excavating the live ones; the hatching
obligation is what inverts the lookup.
