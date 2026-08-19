---
provenance: authored
status: accepted
date: 2026-08-19
---
# 0065 — An ask is owed until it is answered

## Context

Reported from the operator: *"I've already re-typed the same errand five times."*

An errand is the one thing this runtime asks a human to author by hand, and it
was the one thing it kept only in memory — `Shared.errands: Mutex<Vec<..>>` in
the dispatch process, drained with `mem::take`. Four ways a typed instruction
failed to become a session, none of which gave it back:

- **No free slot.** `fire` returns `Deferred` when `capacity() == 0` and logs
  *"deferred, still due"*. It was not still due. A plan stays `ready` and is
  re-scanned next tick; an errand had no such backing store, so the request was
  already out of the queue and nothing put it back. 0064 made this worse: a
  session declaring a wait now holds one of two default slots for hours.
- **A session already on the plan.** Refused 409 with *"retry in a moment"* —
  and nothing held, so retrying meant typing it again later.
- **A dispatcher restart.** The queue died with the process.
- **Invisibility.** The queue was never on the wire. The board inferred "errand
  queued" client-side and reset it on every plan change, so a reload could not
  distinguish an ask still waiting from one dropped an hour ago. Not knowing is
  what produces the fifth re-type.

The runtime had already made the sibling argument and won it: `sessions` and
`pending` are merged live into `/api/status` because *"a question raised between
ticks has to be answerable before the next one."* An ask made between ticks has
the identical claim and never got it.

Worth naming plainly, because it was this decision's own principle turned
against the operator: 0060 held that *"an instruction silently displaced is an
instruction lost"*, and answered it by reporting the displacement. That was the
wrong end of the problem. The commonest reason an operator asks twice is not
that they changed their mind — it is that they cannot tell whether the first ask
survived. Reporting the displacement more clearly does not help someone who is
re-typing because they were never shown the queue.

## Decision

**An ask is never refused for timing and never displaced.** It is queued,
durable, ordered, and visible. The only thing that stops it firing is the plan
changing underneath it — and even then it is held for its author, not dropped.

**The queue is a file.** `.trellis/runtime/errands.json`, saved on every
mutation and atomically renamed, beside the state files. One writer by
construction: `shared.dispatches` is true only in the dispatch process and serve
relays to it, which is what `state.rs` requires of a JSON document. This departs
from that module's posture in one way, deliberately: losing `dispatch-state.json`
is safe because the tree can rebuild it, and losing *this* file is not, because
it is the only record that an ask was made.

**A second ask joins the first.** Replace-by-plan is gone; a plan may carry
several asks and they fire in the order they were made, one at a time, because
`fire` already defers a key in flight. The one thing that does not queue twice is
the same instruction already pending on the same plan — that is recognition of a
double-click, not displacement of an intent.

**A busy plan queues instead of refusing.** The in-flight 409 is removed. One
plan still carries one session; the ask simply waits its turn. The nudge stays,
so an ask lands within a second when the running session has in fact just ended.

**An ask is pinned to the plan it was written about.** `plan_ops::fingerprint`
hashes the plan's body plus the declared fields the ask depends on — `owner`,
`type`, `complexity`, `awaits`, `contexts`, `subdomains`, `metrics`, `tags` — and
explicitly **not** `status:` or `handoff:`, which the runtime writes on its own;
hashing those would invalidate every standing ask the first time anything
dispatched. When the fingerprint moves, the entry is marked `stale`: held, never
fired, and surfaced to its author to confirm (re-pinned to what the plan says
now) or discard.

This is what lets the deferred ask retry **without a bound**. "Put it back and
try again forever" is reckless on its own; bounded by a real signal rather than a
guessed TTL, it is just correct. A clock would have been the outlier here anyway
— `awaits:` releases on a status, `handoff:` is cleared by a claim, the
escalation ledger is keyed by content identity. This codebase derives; it does
not expire.

**The hash is hand-rolled FNV-1a.** This value is *persisted*: an ask queued
under one binary is judged under the next. `DefaultHasher` is explicitly not
stable across Rust releases, so an upgrade would have silently invalidated every
standing ask. Eight lines and a fixed constant, against a dependency this tree
would otherwise have refused.

**The queue is on the wire.** `/api/status` carries `errands`, merged live for
the same reason `sessions` and `pending` are. The board lists a plan's pending
asks with their age, marks the stale ones, and offers the two verbs. An ask
nobody can see is an ask that gets typed again.

## Consequences

- **A backlog can accumulate**, where before a forgotten ask quietly evaporated.
  That is the trade, and visibility is what makes it survivable — which is why
  the wire field and the board list are part of this decision rather than a
  follow-up.
- **A crash between the spawn and the conclusion is repaired, not lost.** An
  errand never claims its plan (only `act` does), so the entry is marked
  `Dispatched { key }` and saved *before* `backend.spawn` — the ordering `fire`
  already uses — and startup reconciles it against the pool's live keys beside
  `marker::reconcile`. An ask whose session is gone goes back in the queue.
- **Durability covers a dispatcher restart, not a dispatcher absence.** The queue
  lives in the dispatch process, so serve still answers 503 when none is running.
  Letting serve write the file instead was rejected: it reintroduces the
  two-writer race, and "the dispatcher is down" is itself racy.
- **The fingerprint is a proxy.** An ask pointing outside the plan — "check the
  CI failure on PR #1228" — goes stale while the plan text, and so the
  fingerprint, stays green. It bounds the queue; it does not validate the ask.
- **It answers coherence, not presence.** 0057 made the operator's request the
  authorization for a session on a human-held owner. An unchanged plan eight
  hours later still fires an ask into a room with nobody in it. Accepted: a late
  ask the operator can see and discard is better than a lost one they cannot.
- **This partially revisits 0060**, which removed `GET /api/errands`. What 0060
  killed was errand *types* — named, config-declared framings — and that stands:
  there is still one errand and it has no name. Reading back *queued instances*
  is a different thing. An in-memory queue draining within one tick did not need
  to be visible; a durable, multi-entry one does.
- The board's drawer no longer closes over a half-written ask. That was the last
  remaining way to lose typing, and it had nothing to do with the daemon.

## Alternatives rejected

**A wall-clock TTL on each entry.** Simpler, and it answers presence rather than
coherence. Rejected as the primary bound because the number would be a guess,
and because every other staleness rule in this runtime is derived from content
or status rather than from a clock. Nothing prevents adding one later if late
asks prove to be a real complaint; the fingerprint does not preclude it.

**Writing the ask into the plan's frontmatter.** The one queue the runtime
already trusts across processes and machines, and durable in git. Rejected: it
reverses 0060 outright and puts transient, unstructured operator instruction into
an authored artifact — a commit for "actually, tighten the scope" — while
creating a second answer to "how does work get dispatched" beside `status:
ready`.

**Keeping the 409 and fixing only the drop.** Half the retyping came from the
refusal, which is the case where the operator is most plainly present and most
plainly ready for the work to happen. Refusing them at that exact moment, with
nothing held, is the behaviour this decision exists to delete.
