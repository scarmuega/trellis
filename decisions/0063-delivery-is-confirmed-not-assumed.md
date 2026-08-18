---
provenance: authored
status: accepted
date: 2026-08-18
---
# 0063 — Delivery is confirmed, not assumed

## Context

Between 0.24.0 and this, auto dispatch started harnesses that were never told
anything. The pane came up, the agent ran, the plan read `active`, and nothing
whatsoever happened in it — for the whole of `idle_grace_secs`, and then for
the whole of `retry_cooldown_secs`, and then again. On a dogfooding root on
2026-08-17 one plan spent an evening being dispatched into silence, four times,
before the fourth attempt could not even start.

Four faults, all at the same seam, none of them visible from the call that
hits it.

**Asking to submit was what kept the agent from being submittable to.** Herdr
refuses `agent.prompt` with `agent_not_ready` until its agent can take input,
and 0.24.0 waited that out by *retrying the submission* twice a second. A
refused submission is not free — it is a UI-changing request — and retrying it
at that rate holds the pane in the state it is waiting to leave. Measured on
herdr 0.8.0 with two otherwise identical starts: retrying `agent.prompt` at
2Hz, the agent was still refusing at sixty seconds; polling `agent.get`
instead, the same start was promptable in three. The runtime spent months
believing the harness was slow to come up when it was the asking that kept it
down.

**And running out of the window counted as success.** The retry budget was
twenty attempts at 500ms — ten seconds — after which the pool logged a line
and returned `Ok`, handing the session over with the prompt unplaced. That
clause was written when a resubmission pass would place the prompt on a later
tick; 0061 deleted the resubmission pass and left the clause. So the two
faults compounded exactly: the asking kept the agent unready, the window ran
out, and the runtime called it a session.

**An accepted prompt is not a delivered one.** Reproduced outside trellis on
2026-08-18, against herdr 0.8.0 and Claude Code 2.1.234: with everything herdr
reports being right — `interactive_ready` true, `agent.prompt` accepted, no
error anywhere — the injected text lands nowhere on some starts, and the pane
sits at an empty composer. This is the seam the pre-0061 machinery existed to
paper over, and it is still there; 0061 removed the paper, having judged that
recycling recovers it. On a live root the recovery reads as a dispatcher that
does not work.

**A recycled plan could not be dispatched again.** The session's herdr agent
was named after its task alone. Herdr refuses a name another agent holds —
including one that has exited — and under `retain = "on-failure"` a recycled
session's workspace stays up *by design*, as the scene of the stall. So its
agent kept the name, and every later dispatch of that same plan died at
`agent.start` with `agent_name_taken`, immediately, until an operator closed
the pane by hand. The one plan most likely to need a retry was the one that
could not have one.

## Decision

**Placing the prompt is a precondition of spawning, not an attempt made on the
way to one.** `spawn` does not return a session until a turn has started. It
waits for the agent to become promptable, submits, and then confirms; if it
cannot get there, the spawn *fails* — the workspace comes down, `fire` releases the claim
and removes the marker, the plan goes back to `ready`, and the retry cooldown
paces the next attempt. There is no longer any path on which a pane is left
running an agent nobody told anything.

**Readiness is asked for, never tried for.** The runtime polls `agent.get`
until herdr reports the pane `interactive_ready` and no longer
`launch_pending`, and only then submits. `agent.prompt` is called at most
three times per spawn, ever — a hard cap rather than a budget, because
submitting into an agent that is not promptable is not merely useless, it is
harmful.

**Delivery is confirmed against herdr's own `agent_status`.** A prompt that
landed is a turn: `working`, or `blocked` at the agent's own dialog, which is a
turn that began and stopped to ask. A prompt that started no turn is submitted
again, up to twice more. Nothing here reads the screen — no scrollback needle,
no status-footer parse, no terminal title. This is the narrowest signal that
answers the question actually being asked, which is *did the text arrive*, and
it is asked once, at spawn, about a pane the runtime has just created.

**One knob bounds the wait.** `harness.herdr.prompt_deadline_secs` (default
120) is how long an agent has to become promptable before the spawn is given
up, so a spawn cannot block a tick indefinitely. Confirming a submission is a
short window of its own per submission and is deliberately not drawn from the
same purse: a slow cold start that spends the whole budget getting promptable
must still have something left to notice a dropped prompt with.

**An unreachable herdr during confirmation is undecidable, not undelivered.**
If the server goes silent between the accepted prompt and the confirmation,
the submission is believed. The two mistakes are not the same size:
resubmitting into a session that did start doubles a turn, while believing one
that did not costs nothing the reap does not already handle — an unreachable
server is a fleet fact there, with its own patience.

**The pane leads the agent name.** A herdr agent is named `t-<pane>-<key>`.
The pane is fresh for every spawn and is the session's identity anyway (it is
what every other call already addresses), so a retained workspace's agent —
live or exited — can never hold the name the next dispatch needs.

## Consequences

**0061 stands, and this is not a retreat from it.** Completion is still read
from the plan on disk, and nothing here touches that: no exit code, no settle
event, no reading of a screen to decide whether work happened. What is added
is at the other end of the session's life, and answers a different question.
0061's own cost ledger named the case — "the fix, if drops prove frequent, is a
disk-first acknowledgment the session writes, not a return to matching
scrollback." Drops proved frequent within a day. This is neither of those: the
acknowledgment is herdr's structured agent state, which the pool already reads
every tick, and no artifact has to be invented for a session to write.

**A spawn can block a tick for up to `prompt_deadline_secs`.** It already
could — `agent.start` is given sixty — and the ceiling is now explicit and
tunable. The dispatch loop is sequential by design; a slow start delays the
pass, it does not lose it.

**A resubmission can double a turn.** If the first prompt lands but its turn
takes longer to start than the confirmation window, the second submission
queues behind it and the act runs twice. The act prompt absorbs this the way
it always has — it is written to be re-enterable, and a duplicate turn finds
the work done and settles. This is the trade the old machinery took too, for
the same reason, and it is far cheaper than the mute session it replaces.

**A drop that survives three submissions is a failed dispatch, not a stall.**
It is reported as such, the plan is queued again within the cooldown rather
than the grace *plus* the cooldown, and the pane is closed rather than left on
screen — an agent that was never told anything is not a scene worth attaching
to. Only sessions that actually ran can now reach the recycle path, which
makes `recycled` mean what it says.

**The runtime is now sensitive to `agent_status` at spawn as well as at reap.**
A herdr that reported `working` for a pane that is not would break delivery
confirmation. That is the same dependency the reap already has, on the same
field, and 0043's protocol pin is what guards it.
