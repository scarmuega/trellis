---
provenance: authored
status: accepted
date: 2026-07-25
---
# 0030 — Execution health is an instrument, not a metric

## Context
Every domain wants the same mechanical reading: are the plans that exist moving?
Counts per status, draft-versus-active mix, throughput, how far along the active
ones are. The reading is portable in a way no business metric is — it computes
from `plans/` frontmatter and git history alone, so one generator serves any
root and a domain has it before it knows which outcome metrics it will own. That
portability is exactly what makes the filing question sharp, and derivable is not
stated (0024): every instance reaches for these numbers and files them by
instinct.

The instinct is `metrics/definitions.md`, and it is wrong in a way the existing
machinery makes visible. A definition there carries a target and an owner, and
plan-effectiveness item 3 binds it in both directions: every defined metric is
ref'd by some active plan, every active plan refs a metric. So filing execution
health as a definition either produces a permanent false `gap` — a metric nobody
moves — or is "resolved" by writing the plan that moves it, and a plan whose
purpose is improving plan throughput is Goodhart with a mandate (split pieces to
raise the closure count, withhold drafts to keep the blocked share low, retire to
clear the board) or rule 8 firing, since a unit needing plans about its plans is
a domain.

Two of the obvious measures are also unsound as stated. Percent-complete has no
field behind it and would have to be self-reported, which is unfalsifiable; its
failure mode is the plan that is ninety percent done for a quarter. And a
fulfillment rate cannot be read off `retired`, which conflates shipped,
abandoned, and superseded.

## Decision
Codify the reading as a `spec/patterns.md` pattern (Execution health), advisory
only: a *census* (plans per `status:`, cut by `type:`, `owner:`, and subdomain
automation class), a *flow* (entries, releases, closures per cadence, plus dwell
— how long a plan has held its current status, counted from the commit that set
it), and a *mix* (status shares, WIP per owner, cycle time as WIP over closure
rate).

**Placement: an instrument, never a metric.** The readings land as a generated
plan board under `metrics/actuals/` — `provenance: generated`, written by the
steward (whose mandate already names plan boards among its generated views),
carrying thresholds that say *look* rather than targets anyone is graded
against, and stale past the sweep cadence like any other reading (rule 5). It is
never a `definitions.md` entry and is ref'd by no plan, so item 3 stays about
what the business moves. No directory is earned: a view over the plan set is a
view (rule 7).

**Percent-complete is rejected outright**; dwell and *movement* (no commit
touching the plan or its declared contexts within a cadence) are the mechanical
substitutes, and a domain buys finer resolution by decomposing (0026) — a family
of six siblings with four retired is a percentage the filesystem computes.
**Closure rate is clearance, not success**, unless the instance adopts a body
convention: an anchored verdict section recorded as the status flips (shipped |
abandoned | superseded), which is what a plan's owed verdict (effectiveness item
13) looks like written down.

Non-normative filing only: spec stays **v14**, no schema change, no new lint
item. The conventions skill gains a placement row so the numbers do not get
misfiled into `definitions.md`.

## Consequences
The board's real use is that the mechanical fraction of the effectiveness walk
stops being re-derived each cadence: ready dwell past the dispatch cadence
(item 18), blocked dwell past a ritual cadence (item 9), active dwell past a
plan's measurement horizon (item 13), and WIP on generic or low-ranked
subdomains (items 6–7) arrive as sorted queues, so the focus role's judgment
spends itself on plans the arithmetic already ranked. `definitions.md` keeps its
meaning — what the business is trying to move — and the steward gains no
authority it did not have.

The limit travels with the pattern and must keep traveling: execution health can
be perfect while the business dies, so it never substitutes for the challenge and
maturity items (12, 14–15). A board trending up while outcome metrics flatten is
the value-dead finding arriving early. An instance that nonetheless wants targets
on throughput is deviating from an advisory pattern, which the patterns preamble
already permits — with a decision record of its own.

## Alternatives rejected
**Define it in `metrics/definitions.md` with targets** — the filing this decision
exists to refuse: it earns the number a plan, and that plan is Goodhart or
rule 8. **A `progress:` field on the plan schema** — a normative schema change to
carry a self-reported number that cannot be checked, when dwell and movement are
already computable and cannot be typed in by wishful thinking. **Splitting
`retired` into `completed | abandoned`** — a spec bump for what a body convention
carries; 0029 is the standard for a new status value (`ready` drives dispatch),
and a completed/abandoned split drives no runtime behavior, while the closure
event itself is already in git. **A `boards/` or `reports/` directory** — a
grouping, not a kind (rule 7), the same violation 0021 and 0026 name. **A lint
item requiring the board** — the pattern is advisory; a root that declines the
instrument should not fail its lint, and a board that exists and rots is already
caught by lint item 12's freshness check.
