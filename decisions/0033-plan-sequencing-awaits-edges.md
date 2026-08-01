---
provenance: authored
status: accepted
date: 2026-07-31
---
# 0033 — Plan sequencing: `awaits` edges as a dispatch signal

## Context
Plan decomposition (0026) gives a large effort flat sibling pieces, each with an
independent lifecycle; plan dispatch (0029) advances every `ready` plan through
a deterministic scan. Between them sits an ordering nothing declared can carry:
the seams between pieces live in the umbrella plan's prose, which the scan
cannot read, and `status: blocked` records a defect with escalation prose a
human must clear — the opposite of "waiting on another plan to finish." An
owner sequencing a decomposed program today either releases pieces by
hand-timing (the owner as scheduler, re-entering on every completion) or
releases them all and lets dispatch burn sessions on plans whose prerequisites
are unfinished — at worst getting a plausible implementation of piece three
against a codebase piece one has not yet changed, the expensive quiet failure
0031 exists to prevent.

The knowledge layer already models this genre of relation — `induced-by:`,
`funded-by:`, `supersedes` — while the execution layer is a flat pool with
statuses and no edges between plans.

## Decision
Add an optional plan field **`awaits: [plans/{other}.md]`** — declared
sequencing between plans. The dispatch scan **holds** a `ready` plan whose
targets are not all `retired`: skipped, still `ready`, no status change,
retried every tick; the hold clears itself when the last target retires. A hold
is never a flavor of `blocked` — `blocked` records a defect a human must clear
and leaves the queue; a hold records an order the owner declared and stays in
it, which is the whole value: no re-release when the wait ends.

**Satisfied means every target is `retired`**, stated honestly against the
status set: there is no `done` (0030 refused splitting `retired`), and under
dispatch a finished plan sits `active` with a merged PR because the verdict is
the owner's (0031). Retirement is therefore the only owner-asserted end of a
plan's lifecycle — making a dependent's release an owner-gated governance act
rather than an inference from forge state, which would drag PR semantics and
judgment into a scan 0029 requires to read declared fields only. The cost — a
finished-but-unretired target starves its dependents on the owner's verdict
latency — is instrumented, not accepted silently: the verdict debt 0030 and
0031 name acquires its first operational consumer, and plan-effectiveness item
20 surfaces the starvation as a `blocker` addressed to the target's owner. The
inverse gap — a target retired as abandoned mechanically releasing the hold —
is judgment, filed as item 21: the mirror of rule 11's re-parenting, since
retiring a plan orphans its awaiters exactly as discarding a strategy orphans
its induced subdomains.

The name is `awaits`, not `blocked-by`: naming the edge with the status's root
word would force every future sentence to say "blocked-by but not blocked," and
rule 10 makes names retrieval keys — one grep must not return two mechanisms
with opposite clearing semantics. An active verb on a same-kind peer edge
follows `supersedes`; the `-by` passives (`induced-by`, `funded-by`) name what
causes or sustains an artifact, which this is not.

This clears the bar 0026 set when it refused `parent:` — "nothing in the lint
or effectiveness walks would consume it" — and the drives-the-runtime bar 0029
set for `ready` and 0032 for `complexity:`: dispatch branches on the edge.
Failure semantics are fail-closed and deterministic: a target that does not
resolve holds the plan and warns (a typo must not void an ordering the owner
declared; plans are never deleted, so permanent wedging requires a lint-visible
modeling error), and the graph must be acyclic — a cycle is a deadlock by
construction, holding every member until the others retire, which none can.

Two coder clauses keep the edge honest. A held plan handed to the coder outside
dispatch is reported and left untouched — flipping it `blocked` would convert a
self-clearing hold into an escalation a human must clear. And the coder **may
propose `awaits:` on its residue draft** — residue most often queues behind the
plan just advanced — while a new hard boundary forbids editing a released
plan's edges: sequencing is the owner's declaration, and an agent that could
drop an edge could route around a hold.

Spec **v15 → v16, additive** — existing plans stay valid; absent means no hold.
No Rule changes: no Rule references plan frontmatter fields, and unlike rules
11 and 12 there is no orphan condition — most plans await nothing. Checks:
conventions-lint item 4 validates resolution (never self, never outside
`plans/`), new item 22 catches cycles in item 21's closed-loop idiom;
plan-effectiveness item 18 gains the held-plan carve-out and items 20–21 land
as above, gated to v16 roots. Eval fixtures stay green: the field is optional,
no fixture declares it, `evals/grade.mjs` parses no plan frontmatter, and the
eval contracts pin items below every appended number.

## Consequences
A decomposed family releases at once and drains in dependency order — the owner
schedules by declaring, not by re-entering. Dispatch stops burning sessions on
plans whose inputs do not exist, and the focus walk gains a critical path: a
held plan is distinguishable from a stalled one (item 18 vs item 20), and
verdict latency — previously a cosmetic debt — now visibly starves a queue
someone owns. Residue drafts can carry their ordering back to the work that
spawned them instead of relying on the owner remembering. The reference
`dispatch.yml` grows a list-valued frontmatter read and one `status:` lookup
per target — more shell, still zero judgment.

The spec bump moves the instance pins (`template/decisions/0000-adopt-trellis.md`,
the eval skeleton's two) and `README.md` from v15 to v16 in the same change, so
a freshly scaffolded root passes conventions-lint item 18 on arrival.

## Alternatives rejected
**Naming it `blocked-by`** — the natural English collides head-on with
`status: blocked`, whose semantics (defect, escalation, human clears it, leaves
the queue) are the opposite of a hold on every axis that matters.
**A `parent:` revival** — ordering is not ancestry: 0026's refusal stands
because an umbrella-child edge is consumed by nothing and says nothing about
which siblings gate which; a plan may also await a non-sibling.
**Reusing `status: blocked`** — conflates defect with sequence, fires item 9 on
healthy waiting, removes the plan from the queue so nothing self-clears, and
re-installs the owner as scheduler to re-release each wave.
**A `sequence:` list on the umbrella** — the scan reads the plan it is about to
fire, not a registry elsewhere; an umbrella-side list is a second authored
hierarchy over the siblings that drifts from the pieces and dies when there is
no umbrella.
**Deriving dispatchability purely from topology** (no `ready`; dispatchable =
dependencies done, the issue-graph tools' model) — removes the owner's explicit
release, which 0029 made a deliberate opt-in, and lets an authored edge enqueue
work no owner released: the closed agent loop 0031 refuses.
**Satisfied ⇔ merged PR or done-in-fact** — forge state and judgment inside the
deterministic scan, binding-specific besides, and self-report by another name
once the heuristic fails (0030's unfalsifiability standard).
**A `done` status** — 0030's rejection stands: the body-convention verdict
carries the shipped/abandoned split, and the predicate needs only the
lifecycle's owner-asserted end, which `retired` already is.
