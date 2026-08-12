---
provenance: authored
status: accepted
date: 2026-08-12
---
# 0048 — Refine is a procedure, not a plane

## Context

Operating from the board surfaced a recurring want: point at one plan and have
its owner reshape it — simplify the scope, split it, refactor its structure —
without opening a session by hand. The work itself is nothing new: a human at
plane 1 can type `/trellis:act <owner> …` with that instruction today. What was
missing is a name for the errand and a way to request it that does not wait for
a human to be in a session. Two questions had to be settled together: where
refine lives in the model, and whether an HTTP request may start it.

On the first, the tempting shape was a sibling primitive beside `act`. That
would fork act's entire discipline — mandate binding, the acting-role marker,
holder adoption, automation classes, escalation — into a second copy that
drifts. The framework already holds the right pattern: `/trellis:ritual` is a
*procedure command* that resolves its own acting role from `rituals.md` and
delegates to act. Refine is the same shape with a different resolver: the
plan's `owner:`.

On the second, 0038 made the serving surface read-only structurally, 0039
refused a cheap ingress endpoint, and the standing rejection in the runtime
companion covers "any write path on the serving surface beyond the single
answer route 0041 buys." 0041's route survived that bar by three claims — tree
read-only, not ingress, not judgment — and 0029 had already ruled that
dispatching a plan "already *in* the domain, not entering it" claims no new
plane. Since 0046, the process that owns session lifecycle — the dispatcher —
runs its own HTTP socket; serve is only the window.

## Decision

**Refine composes act.** `commands/refine.md` ships `/trellis:refine <plan>
[instruction]`: resolve the plan (refusing terminal and ownerless ones), then
perform the `/trellis:act` procedure as the plan's `owner:` with the
refinement contract as input — write target is the plan artifact and any
split-off drafts, never a claim, never the solution. `act` remains the sole
role-invocation primitive; the contract lives at the framework level, not in a
runtime prompt string.

**An operator may request a refine session over HTTP — on the dispatcher's
socket.** `POST /api/plans/{slug}/refine` with an instruction enqueues a
request the dispatch loop drains on a wake (≤ a fraction of a tick); the loop —
still the only spawner — validates against a fresh tree, then fires one
session through the same seam as dispatch, keyed in the same `plan:` keyspace
so a refine and an advance can never run concurrently on one plan. `trellis
serve` answers the same path as a pure relay to `dispatch.addr` (503 when no
dispatcher runs); `trellis dispatch refine <plan> "<instruction>"` is the
headless equivalent. Under 0041's three claims:

- **The tree stays read-only.** The request writes no artifact and no runtime
  file — it inserts a name and an instruction into the dispatcher's memory.
  Every edit happens inside the spawned session, bound by the owner's mandate,
  the gate, and the plan's automation class.
- **Not ingress.** The plan is already in the domain (0029's reduction). The
  instruction is the same input a human types at plane 1's `/trellis:act
  <role> [input]`; the daemon relays it verbatim into the prompt and judges
  nothing. Stated honestly: an instruction is operator text reaching a
  session, so on a non-loopback bind this route is an unauthenticated prompt
  door — the loopback default is load-bearing, and the config comment says so.
- **Not judgment, and serve stays a relay.** The handler validates
  mechanically (exists, not terminal, owned, holder not a declared human, not
  in flight) and enqueues; refusals quote the tree's own vocabulary. Serve
  forwards bytes to the process chartered to spawn.

## Consequences

The runtime companion's standing rejection is amended to carve out this route
beside 0041's answer route, and `server.rs`'s module doc is amended in the
same commit as the route lands, per 0041's precedent. Requests are one-shot
and best-effort: drained on the next pass, re-validated there (a request whose
plan changed underneath it is dropped with a log line — the HTTP reply already
reported the state it was accepted under), and lost on daemon restart, which
costs one more click. A refine session stamps the same per-plan retry state as
dispatch, so a `ready` plan just refined may wait out one cooldown before the
loop's own next attempt — attended dispatch is unaffected. The board's Act
menu is a consumer of this route, not a contract: `trellis dispatch refine` is
the surface that must work. The spec version does not move; this is
binding-owned surface (0038, 0041 precedent), recorded here and in the
template's runtime-binding notes.
