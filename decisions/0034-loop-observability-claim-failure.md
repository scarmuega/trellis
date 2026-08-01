---
provenance: authored
status: accepted
date: 2026-07-31
---
# 0034 — Loop observability: standing loops keep their state in the tree

## Context
The framework keeps re-deriving one principle per feature without ever stating
it: dispatch is idempotent because the claim is a status flip on the default
branch (0029); progress is dwell and movement because percent-complete is
unfalsifiable (0030); the coder's gate flips a doomed plan `ready → blocked`
rather than remembering it was tried (0031); a hold and its release are both
frontmatter reads (0033). Each decision found the same answer — a loop's exit
and progress conditions must be observable in artifacts and git, never in an
agent's memory or its self-report — because each was answering the same
premise: agents are stateless, and structure substitutes for memory (rationale
premise 9), while execution must carry status (premise 5). Derivable is not
stated, and the next ritual, holder, or binding author re-derives it or gets it
wrong.

One instance of getting it wrong already has a gap in the instruments. Dispatch
retries dead sessions by design, and plan-effectiveness item 18 catches the
queue where *no act ever fires*. But a session can fire and die *before the
claim* every tick — acts recorded in the workflow runs, no `ready → active`
flip on the default branch, budget burning on every cadence while the queue
believes the plan is untaken. Item 18's dwell clock looks for absent acts, so
this failure — the loop that spins without progressing — is invisible to it.

## Decision
State the principle as a pattern and instrument its uncovered failure mode.

**`spec/patterns.md` gains a Loop observability entry** (beside Execution
health, its closest kin): every standing loop — a ritual on its cadence, the
dispatch tick, an agent session retried until it lands — keeps its exit and
progress conditions in the tree. It names the existing embodiments (the claim
flip, the hold read, the gate's block, dwell and movement) and ends on the
test: erase every participant's memory between ticks — does the loop still
stop, hold, and resume correctly from the tree alone?

**`checks/plan-effectiveness.md` gains item 19**, the claim-failure finding: a
`ready` plan dispatch has acted on across more than one cadence that never
flipped `ready → active` (or `→ blocked`) — the takers are dying before the
claim; a flip stranded on a proposal branch is the same finding, since the
claim is a default-branch edit. `blocker` to the plan's owner; session-sizing
and wiring causes to the steward. It is deliberately not gated to v16: the
failure exists on any root with dispatch, from v14 on.

Non-normative throughout — a pattern entry and a checklist item; no schema, no
Rule, no spec content beyond the version 0033 already bumps. The pattern
grounds in premises 9 and 5 and needs no new premise.

## Consequences
The principle is citable instead of re-derivable: a new ritual, holder, or
binding checks itself against one pattern rather than reverse-engineering four
decisions. The dispatch instrument set now covers both stall shapes — the act
that never fires (item 18) and the act that fires and dies unclaimed (item
19) — and the second was the expensive one, since it burns budget on every
tick while looking like motion. Eval fixtures stay green: no fixture contains
a `ready` plan, and the eval contracts pin items below the appended number.

## Alternatives rejected
**A lock file or claim marker** — state outside the artifacts, exactly what the
pattern refuses; the status flip already is the lock, and a marker a dead
session leaves behind is a lie the next tick must detect.
**A dispatch-ledger artifact** — git history and the forge's workflow runs
already are the ledger; a second ledger is a generated view nobody asked for
and one more thing to keep fresh.
**The dispatcher pre-flipping `ready → active` before the session starts** —
the scan gaining the taker's pen; a session that then dies leaves a phantom
`active` plan, the worse lie — invisible to items 18 and 19 both, caught only
by movement, a cadence later.
**A Rule** — the corollary in `spec/rationale.md` places obligations in rules
only when instances must comply; this is practice inside the structure,
advisory by the patterns companion's own charter, and its enforcement surface
(the checklist item) is already the non-normative side's instrument.
