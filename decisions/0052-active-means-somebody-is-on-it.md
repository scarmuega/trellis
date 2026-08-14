---
provenance: authored
status: accepted
date: 2026-08-14
---
# 0052 — `active` means somebody is on it

## Context

A plan in the first production domain sat `active` for a day with no session
running and nothing due. Dispatch only enumerates `ready`, so nothing retried
it; the board showed it in flight; the work had in fact stopped. Two faults
compounded, and the session log carried both.

The session had parked background monitors and ended its turn — the harness
footer read `2 shells, 1 monitor still running`, its own task list showed
`Deliver PR and hand off to QA` still open, and the pane sat at its prompt
waiting to be re-invoked. The herdr pool read that pane as `idle`, called the
turn settled, retired the session with exit 0, and closed the workspace over
work that had every intention of continuing.

The plan was then left where the session had put it: `active`, claimed by
nobody. `active` is the one status the runtime has no reading for — a `ready`
plan is dispatched, a `blocked` one carries an escalation a human clears, a
`retired` one is a verdict, a held one clears itself — so nothing in the loop
ever looks at it again. Decision 0029 made the taker's flip the idempotence
guard precisely because the artifact, not the daemon's memory, is the truth;
the cost of that trade is that a taker which claims and vanishes takes the
plan out of circulation with it. The retry cooldown (0046) covers the session
that ends *before* claiming. Nothing covered the one that ends after.

## Decision

**Idle is not done while monitors are armed.** Before believing a settle, the
herdr pool reads the harness's own status footer and counts armed monitors; a
non-zero count is a `Waiting` phase — the session holds its slot, shows as
`[waiting]`, and is never retired there. It leaves `Waiting` the way it
entered: on the next sighting. Three claims:

- **A monitor is a promise to come back**, which is exactly the difference
  between a turn that ended and a session that ended. Background *shells* are
  not counted: a shell may be a dev server outliving the turn that started it,
  and counting it would hold a slot forever on a session that really is done.
- **No clock bounds it.** The work a monitor is minding legitimately takes
  hours — the observed one was a 1.8 GB download. A patience budget here would
  kill exactly the sessions it exists to protect, so `Waiting` behaves like
  `Blocked`: it holds its slot, it is visible, and freeing it is an operator's
  call. `--once` cannot wait that out, so it detaches and says so, as it
  already does for `blocked`.
- **Screen-reading, like the prompt-echo check before it.** The counters are
  read from the footer window only, not the transcript; the parse is `N
  monitor`, never a bare word. A read error errs toward finishing — the check
  adds a reason to wait, never a reason to hang.

**A plan the runtime dispatched is never left `active` with nobody on it.**
When an `act` session ends, the plan is read back: `active` with no declared
`handoff:` flips to `ready`. Three claims:

- **`ready` is what the state already is** — released, and nobody has taken
  it. This is not a new lifecycle move but the removal of a way to fall out of
  the lifecycle, and it restores the invariant the model already assumes:
  `active` means somebody is on it.
- **The rule is the kernel's, not the daemon's** (`plan_ops::relinquish`).
  The daemon reads one declared field and calls it; the boundary in the module
  doc holds — the daemon decides nothing.
- **Only `act`.** That errand was sent to advance the plan; a refine session
  never claimed anything, and demoting a plan a human is driving would be the
  runtime overruling its operator. The retry cooldown then bounds the return.

**A handoff is declared, never inferred.** Plans gain an optional `handoff:`
(spec v19 → v20, additive) naming what an active plan is parked on — a PR
awaiting its owner's verdict, or any ref a human must move. While set, the
plan stays `active` and out of dispatch. The alternative was inference — ask
the forge whether a PR is open — which fails the same test `awaits:` failed in
0033: retirement is the owner's verdict, and a merged PR is the taker's event.
A field the taker writes (`trellis plan handoff`) is one declared-field read
for the runtime and one line of trail for everyone else. Claiming clears it: a
fresh claim is a fresh attempt, and a spent handoff would park the next one.

## Consequences

- The default `act` prompt names the obligation: end with a verdict, and a
  plan left active with no handoff comes back to the queue. A taker that
  ignores it costs a re-dispatch, not a stranded plan.
- Dispatch state records which errand each session ran (`last_errand`), which
  is what makes "only `act`" checkable after the fact. State written before
  this reads as "not act" — the first pass after an upgrade relinquishes
  nothing, which is the safe direction.
- A session that ends with its plan `active` and a stale `handoff:` from an
  earlier attempt would park wrongly. The claim-clears rule is what prevents
  it, and it is the only place the runtime touches the field.
- `[waiting]` joins `[blocked]` as a state the board shows and the fleet count
  includes. Both are honest about capacity: the slot is spent either way.
- The plan-status flip route (0049) already refuses a plan with a session in
  flight; a waiting session is in flight, so an operator who wants the plan
  back takes the session down first. That ordering is the intended one.
