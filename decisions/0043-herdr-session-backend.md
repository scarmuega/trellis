---
provenance: authored
status: accepted
date: 2026-08-06
---
# 0043 — Herdr: the first escalation push, and sessions as panes by choice

## Context
The operator runs a terminal workspace manager — herdr, a persistent server
with a socket API that starts, watches, and classifies interactive coding
agents in panes — and asked whether it could carry the session fleet, so the
daemon stops handling processes itself. Two chartered gaps meet that offer
halfway. 0036 accepted "an escalation record notifies nobody" and chartered a
push added *over* the records; 0038 built the seam and named its intended
occupants — "a webhook, a desktop notification, or a chat adapter" — and
shipped none, deliberately, waiting for a domain to pull. And a running
session is invisible: a log you tail, plus whatever `trellis_progress` it
chooses to report — the channel 0041 built *because* the sessions could not
be seen.

The evaluation returned four findings. Herdr manages *interactive* agents:
its start primitive requires a pane at a shell prompt and detects the TUI;
protocol 19 has no way to supervise the headless one-shot `claude -p` this
daemon spawns, so offloading is not delegation of the current lifecycle but a
switch to a different one. Second, `--max-budget-usd` works only with
`--print`: an interactive session cannot enforce the budget leg of 0032's
complexity→session map — verbatim the finding on which 0041 refused ACP.
Third, herdr is pre-1.0 and its protocol is a moving integer (19 today), an
external pin with no lockstep guard available — 0041's third finding
restated. Fourth, herdr's `blocked` is a *session-level* stall — an agent
visibly waiting at its own UI — which is a different layer from the
plan-level escalation record a mandated session writes.

## Decision
**The process spawner stays the default session transport. Herdr ships as
two adapters over one socket: the first escalation push, and an opt-in
session backend — sessions as panes by explicit choice, with the costs named
at the door.**

**The channel: `[[channels]] kind = "herdr"`.** A record that newly reads
`open` becomes a toast in the operator's herdr session — the desktop
notification 0038's seam was chartered for, ~150 lines and zero crates
later. One configured but unreachable at startup is refused, on the
`{plugin_dir}` precedent: configured-and-dead is wrong, not degraded. One
that dies mid-run is a logged announce failure, never a stopped clock — the
`Channel` contract already said so. The record in the tree stays the system
of record; the toast only ever points at it.

**The backend: `[harness] backend = "herdr"`.** One workspace per session at
the domain root, identity tokens on its pane, the agent started interactive,
the prompt submitted to the running TUI — so the harness template splits into
agent *args* and the prompt the placeholder machinery already renders.
Completion is observed, not returned: one `agent.get` per session per tick,
and `working` settling to `idle` is the finish; exit codes become a
rendering. A session that goes `blocked` toasts once, holds its slot — the
capacity pressure is real and hiding it would misreport the fleet — and
waits for an operator who can now simply attach. Shutdown detaches by
default: the sessions live in herdr, not under the daemon, and the next
daemon *adopts* them back by their tokens — which makes the in-flight guard
restart-durable for the first time. `retain` keeps failed workspaces on
screen and clears clean ones, after snapshotting the pane's tail into the
session log the bookkeeping already expects.

**What the backend costs is refused loudly rather than absorbed silently.**
A herdr template that names `{budget}`, `--max-budget-usd`, `-p`, or
`{prompt}` is refused at startup with the reason: the budget cannot be
enforced interactively, and the prompt is never an argument there. A domain
that opts in trades the spend cap for visibility, attachability, and
restart-durable sessions, and the template says so where the choice is made.

**The boundary holds.** The daemon still schedules, spawns, serves, and
relays — herdr carries and shows, and decides nothing. Herdr's `blocked`
never becomes an escalation record; only the session, under its mandate,
writes those. Plan-dispatch idempotence still lives in the `ready → active`
flip on disk (0029): a lost pane is exactly a killed child.

**The client is the smallest thing that speaks the wire.** Hand-rolled
NDJSON over `UnixStream`, one request per connection because that is the
server's own shape, `serde_json` only — the no-tokio argument of 0038
applied. The protocol pin is guarded at runtime, not in prose: `ping` returns
the server's protocol and a mismatch refuses startup naming both numbers;
response parsing is lenient so herdr *adding* fields never breaks it.

**Pull-triggers, so later moves are chartered rather than improvised:** a
budget expression that works outside print mode (or a domain that has
deliberately dropped per-session budgets) is the trigger for considering
herdr as more than opt-in; 0041's ACP triggers stand unchanged; and if the
protocol pin starts costing — a herdr upgrade refusing startup more than
rarely — the pin gets a compatibility range or the client gets regenerated
from herdr's published schema, whichever is then cheaper.

## Consequences
A second machine-state owner enters: pane scrollback beside `logs/`, herdr's
session list beside `state.json`. The first is mitigated by the retirement
snapshot; the second is read back at startup by adoption and accepted for
what remains. An adopted session's ask-channel is gone — its token died with
the daemon that granted it — so it runs but can no longer ask, which is
exactly a pre-0041 session.

Completion by settling is weaker than an exit code: a session that settles
`idle` having done nothing reads as finished cleanly. The plan's own status
is the truth that matters, and the scan re-dispatches what was never
claimed.

`spec/runtime.md` records the new limits — the uncapped budget under the
backend, blocked detection being heuristic screen-matching unless herdr's
claude integration hook is installed, the pre-1.0 pin — and the escalation
row now names the adapter that ships. No lockstep entry, deliberately: the
pin is guarded by the startup ping, and a prose copy of the number would be
the second encoding the guard exists to prevent.

The refusal message for unknown channel kinds changes from "no adapter
ships" to naming the one that does.

## Alternatives rejected
**Replacing the spawner with herdr** — the four findings, of which the
budget is decisive; and `--once`, CI, and any machine without herdr still
need a daemon that works from one binary and a harness.
**`events.subscribe` for status** — a long-lived stream needs a reader
thread, reconnects, and event/state reconciliation, to learn what `agent.get`
answers on the clock the daemon already has. The filesystem-watcher argument
of 0038, unchanged.
**Running the headless argv inside a herdr pane** to get visibility without
the semantic shift — protocol 19 has no such primitive, and recovering an
exit code through a shell wrapper is the shell-string class of bug the argv
arrays exist to prevent.
**Shelling out to the herdr CLI** instead of speaking the socket — viable
for the channel alone, but the backend needs the socket regardless, and one
client serving both beats a binary dependency whose argv surface is a second
protocol to track.
**A generic `exec` or `webhook` channel kind** — a seam widened without a
concrete puller. 0041 said it of transports: a seam with no adapter and no
charter is infrastructure ahead of evidence. The herdr adapter exists
because this domain pulled for it; the next kind ships when a domain pulls
for that one.
**Waiting for herdr 1.0** — the integration is two adapters behind config, a
version gate, and lenient parsing; the exposure is sized to what a refusal
at startup can contain. Deferring the value until someone else's roadmap
lands would be caution priced above the risk it covers.
