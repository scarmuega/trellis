---
provenance: authored
status: accepted
date: 2026-08-05
---
# 0041 — Sessions talk back: an MCP channel, not a new transport

## Context
`trellis serve` starts a session with one `Command`: argv from a template,
stdin `Stdio::null()`, stdout and stderr into a gitignored log, the child in its
own process group. Then it polls `try_wait()` and learns one thing — an exit
code. The channel is one-way by construction, and three costs follow from that.

A session that meets an underspecified plan cannot ask. The coder's mandate says
block rather than guess, so it flips the plan `blocked`, writes an escalation
record, and exits — correct, and expensive, because the next dispatch cycle is a
cadence away and the answer was often one sentence. An advisory role's findings
strand in a log nobody opens (`spec/runtime.md`, known limits). And there is no
kill path at all: `reap()` and `wait_all()` are the whole lifecycle.

The candidate fix that presents itself is a client-to-agent protocol — ACP,
JSON-RPC over stdio, with `session/prompt` returning a stop reason,
`session/update` streaming the turn back, and `session/request_permission`
flowing forward. It would replace the spawn wholesale. 0038 rejected *embedding
the Claude Agent SDK* on the grounds that it hardcodes one harness into a kernel
whose argument is that a second binding should be cheap. ACP is not that
objection — it is the standard that would make the claim true — so it deserves
its own answer rather than inheriting that one.

## Decision
**The argv spawn stays the session transport. Sessions get a channel back — an
MCP server on the socket the daemon already listens on.**

**ACP is refused, on three findings.** Its own agent registry lists Claude Agent
as ACP-capable *via Zed's SDK adapter*, not natively (Codex CLI and Pi likewise;
Gemini CLI, OpenCode, Goose and ~25 others are native). Adopting it would take
the reference binding from `claude` — one binary the operator already has — to
`claude` plus a Node shim, which is verbatim the failure 0038's context opens
with: a dependency the reference binding never declared it had. Second, ACP has
no budget expression at all, and its model and effort controls are
*agent-defined* option lists discovered at runtime, so the complexity→session
map of 0032 gets harder rather than portable — the one thing it was supposed to
fix. Third, `agent-client-protocol` 2.0.0 shipped a major version inside a year
behind nine `unstable_*` gates, and an external protocol pin is exactly the
second-encoding-of-normative-prose that `lockstep.rs` exists to guard, with no
cheap guard available.

**The pull-trigger is named, so a later adoption is chartered rather than
improvised:** Claude Code speaking ACP natively; *or* a second harness actually
adopted and blocked by the placeholder set's Claude Code vocabulary; *or* the
`mandate.md` → permission-profile item being built, for which ACP is the
transport that project wants.

**What ships instead.** The daemon serves MCP at `/mcp/{token}`, and the session
is handed its own endpoint as one more argv element through the existing
placeholder mechanism. Three tools, and no more: `trellis_ask` parks a question
and returns a ticket, `trellis_await` waits a bounded interval and returns the
answer or `pending`, `trellis_progress` appends a semantic note. Every harness
speaks MCP natively, so this costs no adapter; MCP's streamable HTTP permits a
server that never initiates messages to answer each POST with plain JSON, so it
costs no async runtime and no second listener; and a template that omits `{mcp}`
keeps working, so the channel is opt-in per binding and a harness without MCP
still runs.

**`trellis inbox` is the contract for answering** — the surface that must work.
The board is a glance surface over the same state and explicitly not the
contract. This is not a preference about terminals: the board is a *plan* view,
and only `act` sessions are one-to-one with plans, so a question raised by a
ritual or by the dispatch scan has no card to appear on. A channel whose only
surface structurally cannot represent a third of its senders is not a channel.

**Why this does not cross the boundary 0038 drew.** Three claims, each load-
bearing. The *tree* stays read-only: answers and progress write
`.trellis/runtime/`, never an artifact, and every change to the domain still
enters through a session bound by its mandate, the gate, and the artifact's
automation class. An answer is *not ingress*: 0038 refuses "a call that could
invoke a role", and nothing here invokes anything — the session is already
running under a mandate the daemon did not grant and cannot widen. And relaying
is *not judgment*: `daemon/mod.rs` already charters the daemon to "schedule,
spawn, serve, and relay", and carrying a question to a human and the answer back
is that verb exactly. The daemon never answers; it only knows where to put the
question.

## Consequences
The daemon gains its first non-GET route, `POST /api/sessions/{token}/answer`.
The blanket 405 stays for everything else, and `server.rs`'s module doc is
amended in the same commit rather than left claiming a property the code no
longer has.

Where there is no socket — `--no-http`, or `--once`, which is a cron-style pass
with no operator watching and would collide with the daemon it most likely runs
beside — `{mcp}` renders to a config naming no server, and the daemon says so
once at startup. A warning, not a refusal: the `{plugin_dir}` precedent refuses
because a session pointed at the wrong plugin is *wrong*, whereas a session that
merely cannot ask behaves exactly as every session did before this decision.
Refusing would also break `serve --once --no-http` for a default config that now
names `{mcp}`, which is a working cron mode and not a misconfiguration.

The session token is **attribution, not authentication**. It says which session
is calling so a question can name its plan; it is not a secret, and the surface
carries no auth in either direction. That is the threat model the API already
had, and binding anything but loopback remains a deliberate act.

A parked session holds a worker while it waits, so the server's thread pool is
sized for it. The wait is bounded and the agent polls, rather than one call
blocking indefinitely — harnesses impose tool timeouts, and a bounded poll is
timeout-safe everywhere without per-harness environment tuning.

New limits, recorded rather than left to be discovered: a question nobody
answers still ends in the session deciding for itself, so the channel shortens
the loop and does not close it; progress is what the session chooses to report,
not an observed event stream, so a session that reports nothing looks idle; and
under `--once` an unanswered question keeps the pass waiting on its session.
Cancellation lands here too, as a signal rather than a route — the children have
been process-group leaders since 0038, so `kill(-pid, SIGTERM)` was always the
whole feature and never needed a protocol.

The spec stays v16. `spec/model.md` is untouched: this is binding-owned
consumer tooling on 0038's own precedent, and `spec/runtime.md` carries the
binding table row and the new limits.

## Alternatives rejected
**Switching the spawn to ACP** — the three findings above, of which the Node
shim in the reference binding is decisive. Recorded with a pull-trigger, not
closed.
**ACP behind a dual-transport seam, shipped now** — the `channels.rs` shape,
and it does not transfer: 0036 had *chartered* that seam before 0038 built it,
and nothing has chartered this one. A seam with no adapter and no charter is
infrastructure ahead of evidence wearing a pattern's clothes.
**Trellis as an ACP *agent*** rather than a client — `act` exposed as
`available_commands`, driven from any of ~30 editors. Genuinely interesting, and
the direction where ACP's ecosystem meets this project's portability claim; it
binds the `session` service rather than the spawn, which is a different decision
than this one. Left open deliberately.
**Hand-rolling JSON-RPC over stdio to the agent** to get ACP's shape without its
dependencies — drift risk on a spec this repo does not own, with no lockstep
guard available, which is the third finding restated as a build.
**A filesystem question protocol** — the session writes a file and polls for an
answer. Universal and dependency-free, and worse: the agent burns turns polling
where a tool call simply waits, and the daemon gains a second writer to its own
runtime directory.
**The board as the answering contract** — it cannot represent ritual or
dispatch sessions, and a 10-second poll over a whole-tree recompute is the wrong
loop for an interaction. It stays the glance surface, which is what it is good
at.
**Parsing session logs for structured output** — `--output-format stream-json`
would give a full event stream for free on one harness. Deferred, not refused:
land the portable channel first and find out whether agent-reported progress is
already enough. If it is not, the parser is an adapter and a missing one
degrades to today's bytes.
**A cancel route on the HTTP surface** — the answer route is the only write
path this decision buys. Widening it needs its own evidence.
