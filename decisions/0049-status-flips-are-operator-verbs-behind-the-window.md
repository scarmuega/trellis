---
provenance: authored
status: accepted
date: 2026-08-12
---
# 0049 — Status flips are operator verbs behind the window

## Context

The board can read a plan's lifecycle and, since 0048, ask its owner to
reshape it — but moving the lifecycle itself still meant a shell: `trellis
plan release | claim | unblock | retire`. Those verbs are the operator's own
(plan_ops: "who may flip is the invoker's mandate, not the tool's business"),
they carry their gates with them — the mechanical readiness share on release,
the hold check on claim, per-transition source-state guards — and the human at
the board is the same invoker as the human at the shell. The question was
where a flip may enter over HTTP, given that serve is read-only structurally
(no write path exists in that process) and that the mcp/answer routes write
`.trellis/runtime/` and no artifact.

## Decision

`POST /api/plans/{slug}/status` carries the flip, honored only on the
**dispatcher's socket** and answered by serve purely as a relay to it — the
same shape 0048 settled for refine. Three claims, argued the same way:

- **Serve stays read-only, structurally.** The serve process still holds no
  write path; it forwards the request and the answer verbatim, and answers
  503 naming `trellis dispatch run` when no dispatcher runs. The process that
  performs the edit is the one already chartered to write during operation —
  the dispatcher stamps acting-role markers (0045) and owns `state.json`.
- **Not a new authority.** The route performs exactly the guarded edit the
  CLI verbs perform, gates included: release refuses on a failing mechanical
  readiness share unless the body says `force: true` (the CLI's `--force`,
  spelled in JSON), claim refuses a held plan, blocked is refused outright
  (blocking is an escalation with a record, `trellis plan block`, not a bare
  flip), draft is unreachable (a draft is where a plan starts), retired stays
  terminal. An unauthenticated socket performing operator verbs is exactly as
  load-bearing as 0048 found it: the loopback default is the authentication.
- **The flip does not fight the loop.** A plan whose `plan:` key is in flight
  is refused (409) rather than flipped under a running session — with the
  same nudge 0048 uses, so a session that actually ended is reaped and a
  retry lands in a second. A successful flip nudges a pass too: a plan
  released into an agent-operated domain is released to its agents, so a due
  dispatch may fire on it now rather than a tick later. That immediacy is the
  point of the verb, not a side effect to apologize for.

This is the first HTTP route that edits an artifact — one frontmatter scalar,
through the same `plan_ops` guard as the shell, on the operator's own asking.
The line that holds is not "no route writes"; it is that no route grants what
the invoker's plane 1 did not already have, and no route spawns or judges.

## Consequences

- `spec/runtime.md`'s serving-surface paragraph and `server.rs`'s module doc
  are amended in the same commit as the route (the 0041 precedent).
- The board's status chip becomes the menu of legal moves for the plan's
  current status; refusals surface verbatim, and a readiness refusal offers
  the same force the CLI does.
- No new CLI surface: `trellis plan release | claim | unblock | retire`
  already are the plane-1 verbs; the route is those verbs behind the window,
  not a second vocabulary.
- Flipping to `blocked` stays out of the route until the board can carry the
  escalation record a block requires — a block without its record would be a
  vocabulary the tree refuses everywhere else.
