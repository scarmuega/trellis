---
provenance: authored
status: accepted
date: 2026-08-17
---
# 0061 — Completion is a verdict on disk, and herdr is the only backend

## Context

0043 shipped herdr as an *opt-in* session backend beside the process spawner,
and the spawner stayed the default because it had exit codes and budget caps.
The `Backend` interface therefore kept the spawner's shape: `reap()` returns
`Finished { exit }`, a completion *event*, because that is what a `fork/exec`
naturally produces.

The herdr pool had no such event to return, so it manufactured one. By this
month that machinery was: a prompt-echo needle matched against ten thousand
lines of scrollback (to tell a turn that ran from an injection the TUI
dropped), a two-token parse of Claude Code's own status footer counting armed
monitors (0052, to tell a settled turn from a settled *session*), a
three-second blocking sleep re-reading each settle to see whether it survived,
a resubmission budget with its own backoff, an `agent.wait` drain with an
unexplained hundred-round cap, and error classification by substring-matching
message text — including, in `is_unreachable`, matching trellis's *own* format
string. Every one of those existed to answer "did the session finish", and
every one answered it by reading another program's screen.

Meanwhile the answer was already on disk and already authoritative. 0029 made
the `ready → active` flip the idempotence guard *because* the artifact, not the
daemon's memory, is the truth in a stateless git runtime. 0053 moved that flip
ahead of the spawn. 0052 named the invariant outright — "`active` means
somebody is on it" — and then enforced it with a screen scrape. The herdr
module's own doc conceded the point in a sentence: "the truth about the *plan*
stays where it always was — in the status its taker flips on disk." And the
manufactured exit code was already vestigial: `state.last_exit` was written
and read by nothing — not Rust, not the board, not the API. A session that
errored and one that succeeded were bookkept identically, in production, for
months, unnoticed.

The gap was never that herdr reports the wrong thing. Herdr reports *screen
state*, honestly and well. The runtime wanted *turn state*, and had chosen to
want it by keeping a backend interface shaped like a process.

## Decision

**A session is done when its plan says so.** Each tick, for each tracked pane,
the pool takes one coarse `agent.get` and — only for a pane that has settled —
asks its caller what the plan behind that key reads on disk. A verdict there
(moved or retired, blocked, passed on, or parked behind a declared `handoff:`)
retires the session. No verdict, and the pane is a stall: it is granted
`idle_grace_secs`, then **recycled** — `plan_ops::relinquish` hands the plan
back to `ready`, the pane is retired, and the existing retry cooldown paces the
next attempt.

**Herdr is the only backend.** The process spawner is deleted, with
`[harness] backend`, `act_cmd`, and `ritual_cmd`. This supersedes only 0043's
*default*: everything 0043 decided about why herdr — the two adapters, the
protocol pin, the refusals at the door — stands. What it removes is the
alternative, and with it the second lifecycle the `Backend` enum existed to
straddle. `--once`, cron, and CI now require a running herdr server, refused at
startup when it is configured and dead — the `{plugin_dir}` posture, applied to
the one dependency the runtime now has.

**Parking is declared, never inferred.** The footer scrape and its `Waiting`
phase are gone. A session that parks work outliving its turn writes
`trellis plan handoff`, which the act prompt now says in as many words. The
gain is not only the deleted parse: a declared handoff *frees the slot*, where
`Waiting` held one forever with no clock on it at all — 0052 said so and
accepted it, because a monitor may be minding an hour-long download. Reading
the intent off the plan costs nothing and bounds the fleet.

**Recycling is automatic.** No operator escalation for a dead-idle session: it
is closed and its plan requeued. The retry cooldown is re-stamped at the
*conclusion* rather than the spawn, because the retry begins when the plan
returns to the queue, not when the session that stalled first started — which
may have been hours earlier.

**The resubmission machinery is deleted outright, with nothing in its place.**
A prompt the TUI dropped produces a pane that did nothing, which is
indistinguishable from — and recovered identically to — a plan nobody
accounted for. This is the trade taken most deliberately; see below.

**Two counters replace two sleeps.** A settle is acted on at the second
consecutive idle sighting and a block is toasted at the second consecutive
blocked one, both paced by the caller's tick rather than by a blocking sleep in
the reap path. The flaps these guard are real and observed — a permission
dialog reads `idle` for a beat before the classifier calls it `blocked` — but
one tick of latency is a cheaper answer than three seconds of a held thread,
and the reap path now never sleeps.

**Errors are typed.** `HerdrError` is `Unreachable | Refused { code } |
Protocol`. The distinction that earns its keep is the first: unreachability is
a fact about the *fleet*, counted once per pass against `SERVER_PATIENCE`,
while a refusal is a fact about one request and carries herdr's own code, so
`agent_not_ready` and `agent_pane_busy` are retried by name rather than by
grep.

**The claim's crash window closes.** `fire` now orders claim → marker →
`state.fired` + eager save → spawn. The marker and the errand stamp are what a
later startup needs to recognise an abandoned claim; written *after* the spawn,
as they were, a crash during `agent.start` — which may take a full minute —
left an `active` plan that no reconcile would ever see. The remaining window is
two syscalls.

## Consequences

The cost ledger, plainly:

- **Recycle latency.** A dropped prompt used to be recovered in about ten
  seconds; it now costs the idle grace plus the retry cooldown — two minutes
  plus fifteen, at the defaults — and burns a workspace-create per attempt.
- **A dropped ritual prompt is a missed run.** A ritual leaves no artifact to
  read a verdict from, so `NoPlan` settles on the grace alone, and `last_fired`
  was stamped at spawn. For a weekly row that is a week. This is the sharpest
  edge of deleting resubmission and it is accepted with it named: the fix, if
  drops prove frequent, is a disk-first acknowledgment the session writes, not
  a return to matching scrollback.
- **A verdict written mid-turn, then more work.** The two-tick idle streak is
  the only thing between a session that retires its plan and keeps composing
  its report and a pane closed under it. If that proves too thin live, the knob
  is the streak count — not a new heuristic.
- **`--once` and cron hard-require herdr**, including under `--dry-run`, which
  still handshakes. Configured-and-dead is wrong, not degraded.
- **Budgets are gone as a mechanism, not merely unenforced.** There is no
  backend that can cap a session, so `budget_enforced` is now constantly false.
  `{budget}` stays a legal *prompt* placeholder — a session may be told its
  ceiling — and stays refused in agent flags. 0043's pull-trigger stands: a
  budget expression that works outside print mode reopens this.
- `state.last_exit` becomes `last_outcome`, a `Reason` word. Both files are
  `#[serde(default)]`, so old state degrades to absent rather than failing.
- The fake herdr in the test suite gained a session hook: a closure that runs
  while the daemon is blocked in `agent.prompt` and writes the plan the way a
  real session would. Every plan-lifecycle scenario that used to ride a shell
  harness now rides that, which is a truer fixture — the verdict is the thing
  being tested, and it is now the thing the fixture writes. The suite also
  pins `HERDR_SOCKET_PATH` to a dead path, so a misconfigured test can never
  reach the operator's own fleet.

## Alternatives rejected

**An MCP `trellis_done` tool** — a positive completion signal the session
declares, with the disk invariant as a backstop. Rejected: a tool call is a
claim about work in flight, and the plan is already the claim, written through
the same guarded verbs the interactive plane uses. A second path would have to
be maintained, could not be taken by an adopted session (whose token died with
the daemon that granted it), and would be exactly the sort of thing a
misbehaving session gets wrong — while the plan flip is the thing the session
must get right anyway.

**Keeping the process backend, retargeted onto the same rule.** Its exit codes
are honest, and it needs no herdr. But maintaining two backends against one
lifecycle is what shaped the interface into an event in the first place, and
the second backend's honesty was not being used: nothing read the exit code.

**Keeping the footer scrape as a safety net beside the disk rule.** It would
still be Claude-Code-specific screen parsing, still silently wrong for any
other `kind`, and it protects only sessions that decline to declare what they
are doing.

**Escalating instead of recycling a dead-idle session.** Conservative, and it
puts a human in the loop for the most mechanical failure the runtime has. A
stalled session holding a slot until someone notices is the strand 0052 exists
to prevent, arriving by another road.

**Refusing a short `idle_grace_secs`.** A floor would guess at what the
operator meant, which is the habit this decision exists to break. Below sixty
seconds the daemon says so at startup and runs it.
