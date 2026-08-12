# Trellis — runtime companion

> How a Trellis domain is *operated* day to day. Non-normative companion to
> `spec/model.md`: the operating model's contract ends at the root (rules 1–2);
> everything here is consumer tooling. An instance declares which binding it uses
> in its `conventions.md` ("Runtime binding" section); deviations need no spec
> change, only a decision record in the instance.

A runtime is whatever executes the operating model: loads artifacts as
instructions, invokes mandated roles, runs rituals on cadence, routes external
events, enforces authority, and channels escalations to humans. This document
defines the shape any runtime must have — the trigger planes, the one primitive
they share, and seven contract services — then specifies the **reference
binding**: Claude Code plus a git forge, and the **local binding**: `trellis
serve`, the kernel's daemon mode, for a domain operated without forge coupling.

## Trigger planes

Every way work enters a domain is one of three planes:

1. **Interactive** — a human opens a session at the domain root: edits artifacts,
   triggers skills, invokes roles ad hoc. The human is present; the harness's own
   permission model is the gate.
2. **Scheduled** — `rituals.md` rows executed headlessly on their cadence by
   their executor role. No human is present; deviations travel the escalation
   channel and respect the SLAs declared in `rituals.md`.
3. **Event-driven (ingress)** — an external event (support ticket, inbound
   email, webhook) invokes a mandated role with the event as input. Event
   content is untrusted input by default (see the instance's secrets policy);
   the mandate defines what the role may act on.

## The `act` primitive

All three planes reduce to `act(role, input)`:

1. **Resolve** `org/{role}/mandate.md` in this root — never a mandate from
   anywhere else (rule 4).
2. **Bind the holder**: a local package (`holder/system.md` plus its tools,
   skills, evals) becomes the acting identity; a `holder/ref.md` to an external
   agent delegates to that agent; a ref to a *human* is never impersonated —
   work that lands on a human holder becomes an escalation or handoff.
3. **Execute within authority**: `scope:` bounds which artifacts may change;
   `authority:` bounds spend, publishing, and approvals; each touched artifact's
   effective automation policy (below) bounds the mechanics of the change.
4. **Escalate beyond it**: anything outside authority goes to `escalate-to:`
   through the instance's escalation channel — it is never silently attempted.
5. **Leave a trail**: every action is attributable in the ledger to the acting
   role and its trigger.

The interactive plane invokes `act` ad hoc; a ritual is `act(executor,
procedure)` on a timer; ingress is `act(role, payload)` on an event.

## Plan dispatch

A plan flagged `status: ready` is one its owner has released for a taker to
advance. The dispatcher **enumerates the `ready` plans continuously** — a
deterministic scan, no judgment, polled on a tight tick rather than a
calendar (v19; decision 0046): a ritual's time gap is part of its meaning,
but dispatch's old daily cadence was a fossil of the forge cron it was ported
from, and an `awaits:` hold releasing mid-day is precisely the event dispatch
exists to react to. For each dispatchable plan it invokes
`act(plan.owner, "advance plans/{slug}.md")`. The holder branch of `act` (step 2
above) routes it: an agent-package owner advances the plan autonomously; a
human-held owner receives it as a handoff, never impersonated. The taker flips
the plan `ready → active` as it claims the work, so a plan leaves the queue the
moment work starts — dispatch is **idempotent** across ticks without a separate
lock, and a session that dies before claiming simply gets retried next tick. A
plan may also declare `complexity:` (schema in `spec/model.md`); the scan reads it
to size the taker's session — how much reasoning, which model, how much budget —
per the binding's mapping. Reading a declared field keeps the scan deterministic;
absent means the binding's default.

A plan may declare `awaits:` (schema in `spec/model.md`): the scan **holds** a
`ready` plan whose targets are not all `retired` — skipped, still `ready`, no
status change, retried every tick until the last target retires. The hold is not
`blocked`: no defect, no escalation, and it clears itself. Retirement being the
owner's verdict, releasing a dependent stays an owner's act — never an inference
from a merged PR or a session's report. Both reads are declared fields, so the
scan stays deterministic; a target that does not resolve holds the plan and
warns — the lint owns the repair.

Two more gates run before a session is spent, both deterministic, both holds
(v19; decision 0045). The scan runs the **mechanical share of the readiness
gate** — the same checks the taker's pickup gate would bounce the plan on:
declared refs resolve, the automation class is computable, the owner is real,
no deferral tokens in the body — and holds a `ready` plan that fails it,
naming the failed items. Like an `awaits:` hold it writes nothing and clears
itself the moment the plan is fixed; the judgment items stay the taker's, and
the block-flip-plus-escalation ceremony stays a mandated session's — the
precheck only stops the scan from spawning a session whose entire work product
would be discovering the failure. And a role whose `holder/ref.md` declares
`kind: human` (schema in `spec/model.md`) gets a **handoff instead of a
session**: one declared-field read, the same routing act's holder branch
performs, done before a harness is paid for it. Undeclared holders route
exactly as before.

This adds no plane and no contract service: plan dispatch is `schedule` + `act`
composed — the action-sibling of the `focus` ritual (focus *evaluates* plans and
escalates; dispatch *advances* them). Change mechanics still bind: a change to a
`core` subdomain is opened as a proposal, not landed (automation policy below).

## Contract services

| service | obligation |
|---|---|
| **session** | interactive operation at the root: artifacts load as instructions, skills and roles are invocable |
| **act** | the primitive above, invocable from all three planes |
| **schedule** | execute each `rituals.md` row on its cadence, headless; respect metric freshness windows (spec rule 5) |
| **ingress** | route an external event to a mandated role; event content treated as untrusted input |
| **gate** | enforce provenance rules, decision append-only (spec rule 6), and automation policy — deterministically where possible |
| **escalation** | deliver escalations to the `escalate-to:` role's holder with an audit trail; approvals reach humans on a channel they already watch |
| **ledger** | every change attributable: who (role), why (trigger), what (diff) |

## Automation policy → change mechanics

The spec's automation policy (class on the strategy edge, strictest across
committed edges, orphans default to core) maps onto concrete change mechanics.
In forge terms:

| effective class | mechanics |
|---|---|
| `generic` | agent commits autonomously |
| `supporting` | agent commits or merges; humans sample asynchronously on the cadence in `rituals.md` |
| `core` | agent never lands the change itself: it opens a proposal (PR); a human owner review is structurally required |

`owner:` frontmatter is the source for structural review: a generated CODEOWNERS
view (steward territory — `provenance: generated`) plus branch protection makes
"human review gate on every core change" a property of the forge, not of agent
obedience.

## Conforming binding checklist

A binding names, for its harness and forge of choice:

- how a session binds to a root (plane 1),
- how `act` is invoked interactively, headlessly, and from an event,
- what schedules rituals and where escalations land (planes 2–3),
- which planes it leaves **unbound** — an absent plane is a binding choice to
  state, not a gap to discover,
- which enforcement is deterministic (hooks, branch protection, CI) and which is
  prompt-borne — stated honestly,
- where credentials live (never in the root — secrets policy),
- how actions are attributed in the ledger.

An instance records its binding choices in `conventions.md` under "Runtime
binding".

## Reference binding: Claude Code + the `trellis` runtime

The harness runs the sessions; the runtime is three processes with three
shapes (decision 0046) — a continuous dispatcher, a cron-shaped rituals pass,
and a read-only server — and a git forge carries review. The forge is not the runtime: it holds the ledger and the
approval gate, and nothing schedules or triggers from it (decision 0039).

| service | binding |
|---|---|
| session | Claude Code at the domain root with the `trellis` plugin; plan authoring rides the harness's plan mode via `/trellis:plan` (`commands/plan.md`) and persists to `plans/`; plan-effectiveness review via `/trellis:focus` (`commands/focus.md`); plan refinement via `/trellis:refine` (`commands/refine.md`) — `act(owner, refinement)` over a plan's content, never its execution (decision 0048) |
| act | `/trellis:act <role> [input]` (`commands/act.md`) interactively; headless, the dispatcher renders that same command file into the spawn prompt (decision 0050) — computed facts first, the procedure body after — so the spawned session needs no installed plugin, and any harness that takes a prompt can carry it. A domain preferring the slash spelling restores it in `runtime.toml`'s `[prompts]`; `--plugin-root` swaps a live checkout in place of the embedded procedure text, and `--plugin-dir` remains the uninstalled-checkout option for the hooks and skills the interactive plane still needs. Where the template names `{mcp}`, the session is also handed a back-channel to the daemon that spawned it (decision 0041). With `[harness] backend = "herdr"`, sessions run instead as interactive agents in herdr panes — attachable, visible, alive across a daemon restart, and budget-uncapped, the trade decision 0043 records |
| schedule | split by what the gap means (decision 0046): `trellis dispatch run` is the continuous pull loop — every tick it polls the tree and spawns one act per dispatchable plan, no calendar gate, a retry cooldown as the crash-loop backstop; `trellis rituals` is one idempotent pass of the wall-clock work — fire what the cadences owe today, drain, exit — invoked by cron, launchd, or a hand, as often as they like; `trellis serve` carries no clock at all |
| ingress | **unbound.** No event-driven plane ships: an outside event reaches the domain when a human brings it into a session. The daemon's HTTP surface is read-only by construction and is deliberately not a trigger door — a call that could invoke a role would be a plane with no mandate behind it. Requesting an errand over a plan already in the domain is not ingress: no event enters, and the mandate is the one the plan's `owner:` already declares (decisions 0029, 0048, 0051). An instance that needs a real ingress binds it and records the choice |
| gate | plugin hooks (`hooks/hooks.json` → `trellis gate`, falling back to `hooks/gate.mjs` where the binary is absent): deterministic guards on Write/Edit — no hand-edits to `provenance: generated`, no edits to committed accepted decisions, frontmatter warning on new artifacts — plus branch protection + generated CODEOWNERS for core-class review |
| escalation | escalation records in the root: a fenced `yaml` block under `## Escalations` in the artifact the escalation concerns, written by that artifact's owner (schema in `template/conventions.md`); read from the daemon's board and `/api/escalations`, or the root itself; approvals are PRs. One push adapter ships: `[[channels]] kind = "herdr"` turns a record that newly reads `open` into a toast in the operator's herdr session (decision 0043) |
| ledger | git history; the acting role is recorded in the session marker and named in commits. Per-session logs under `.trellis/runtime/logs/` are the daemon's trail — gitignored, single-machine, never the ledger |

**Plan dispatch (`trellis dispatch run`).** The dispatcher polls the tree
every tick and starts one headless act session per dispatchable plan —
`advance <plan>` as its `owner:`, the act procedure rendered into the prompt
(decision 0050) — one act per plan, for clean per-owner attribution, no
calendar gate (decision 0046). The `plan dispatch` row in `rituals.md` stays
as the operator-of-record declaration and the runner skips it: the loop is
continuous, not cadenced. The steward is that row's operator, but because the
scan carries no judgment it is implemented as this wiring, not a steward
session (the steward mandate's own "extract the deterministic parts into
tooling"). The scan maps the plan's `complexity:` tier to the session's
model, effort, and budget — the tier's meaning is the spec's, the prices are
this binding's, retuned in `runtime.toml`, never in a plan — holds on
unretired `awaits:` targets and failed mechanical readiness, short-circuits
declared-human holders into handoffs, and carries the computed facts into the
prompt (`{escalate_to}`, `{automation}`, the pre-verified gate). The
dispatcher owns the `.trellis/acting-role` marker for the sessions it spawns
(decision 0045) and hosts the sessions' MCP back-channel on its own loopback
socket — the channel lives with the process that owns the session lifecycle.
A concurrency cap bounds how many sessions run at once; a **retry cooldown**
bounds how soon a plan whose session ended without claiming may be retried —
the daily cadence used to be that backstop by accident, the loop needs it on
purpose. Reports are transitions, not standing facts: a hold is announced
when it appears and when it clears, never once per tick.

**Operator errands (decisions 0048, 0051).** Every session the runtime
spawns is `act` under some framing, and every framing is a template: the
`[prompts]` table in `runtime.toml` maps errand names to prompt templates,
three of them shipped as framework-authored defaults — `act` (the dispatch
loop's), `ritual` (the cadence pass's), and `refine`, the shipped
operator-requestable errand: `act(owner, refinement)`, a contract whose
write target is the plan artifact (plus split-off drafts) and nothing else,
carried in its default template. Any additional key an instance declares is
a new requestable errand, no recompile — the instance owns its templates the
way it owns its `rituals.md` rows. Interactively, refine is plane 1
(`/trellis:refine`). Headlessly, an operator requests any errand of the
running dispatcher — `trellis dispatch request <errand> <plan>
"<instruction>"` (`dispatch refine` is the alias), or `POST
/api/plans/{slug}/errands/{name}` on the dispatcher's own socket
(`…/refine` aliased), which serve answers only as a relay, and `GET
/api/errands` lists — and the loop, still the only spawner, validates
against a fresh tree and fires one session through the same seam as
dispatch, in the same per-plan keyspace so an errand and an advance never
overlap. `act` and `ritual` stay the triggers' own, never requestable. The
instruction rides verbatim into the prompt; on a non-loopback bind that is
an unauthenticated prompt door, which is why the loopback default is
load-bearing.

**Rituals (`trellis rituals`).** One pass of the wall-clock work: compute the
due set from `rituals.md` cadences and the last-fired dates, fire one session
per due row, drain them, record, exit. Idempotent per day, so the OS may
invoke it hourly and it only fires what is owed — cron and launchd are better
wall clocks than a sleeping daemon ever was, and `catchup` decides what a
window missed while nothing was running does. It opens its own ephemeral
back-channel for the sessions it drains, and its state lives in its own file
(`rituals-state.json`), because two processes on one document is a race.

**The serving surface (`trellis serve`).** Read-only over HTTP, and nothing
else: the computed facts the commands print, the generated views, the open
escalation records, an embedded plan board, and the dispatcher's `status.json`
snapshot overlaid on `/api/status` — labelled with whether that process is
still alive. Read-only is structural: there is no write path, so every change
still enters through a session bound by its mandate, the gate, and the
artifact's automation class. The one non-read it answers besides 0041's
answer route is the errand request (decisions 0048, 0051), and only as a
relay to the dispatcher's socket — serve itself spawns nothing, and answers
503 when no dispatcher runs. It binds loopback by default, carries no authentication,
and may be down without stopping anything. The other non-read it relays is
the status flip (decision 0049): `POST /api/plans/{slug}/status` performs, on
the dispatcher's socket, exactly the guarded lifecycle move `trellis plan
release | claim | unblock | retire` performs — readiness gates release (with
the CLI's own force override), holds gate claim, a plan with a session in
flight is refused, blocked and draft are not reachable by flip — so the
window grants the operator nothing plane 1 did not already.

**Implementation holders.** A plan whose `contexts:` land in code needs a holder
that can write it, and that holder is domain-local (decision 0047, superseding
0031's plugin-shipped one): an implementation role — a local mandate scoping the
subdomains and contexts it may change, plus a `holder/system.md` package with at
least one eval — named as the `owner:` of code-bearing plans; dispatch then
routes to it through the package branch of `act`, with no extra wiring. The
discipline the package carries: block on a specification gap rather than guess,
deliver code as a PR (draft while unfinished) and never merge it, file residue
as a `draft` plan. The class still decides who lands the change: `generic` and
`supporting` PRs may be auto-merged by this binding, `core` waits for its owner.

**Escalations.** This binding keeps escalations *in the tree*: the artifact that
carries the problem carries the escalation, as body content its owner authored
(decision 0036). A blocked plan therefore states its own blocker, so what stops a
loop is reconstructible from a checkout — the Loop observability pattern applied
to the escalation channel, where a forge thread is state filed outside the tree.
Because it is authored content, only the artifact's `owner:` writes a record: an
agent acting *as* that owner (an implementation holder under dispatch) writes its
own, while an
advisory role that owns nothing — `org/focus`, `org/steward` — reports its
findings for the owner to transcribe, which leaves both roles' write boundaries
exactly as they were. An escalation with no artifact to sit in stays in the
acting session's report.

**Asking rather than blocking.** The daemon serves MCP at `/mcp/{token}` and
hands each session its own endpoint as one argv element. A session that meets an
underspecified plan can `trellis_ask` a question with options and
`trellis_await` the answer, instead of flipping the plan `blocked` and waiting a
full cadence for a one-sentence reply; it can `trellis_progress` a semantic note
so the operator sees what it is doing without opening a log. `trellis inbox` is
the surface that answers — the board shows the same state but cannot represent
ritual or dispatch sessions, which have no plan card. Every harness speaks MCP
natively, so this costs no adapter, and a template that omits `{mcp}` — or a
daemon with no socket to call back on, which is `--no-http` and also `--once` —
runs exactly as before: the channel is opt-in per binding, and its absence
degrades rather than refuses. It shortens the loop
rather than closing it — a question nobody answers still ends in the session
deciding for itself, and blocking with an escalation record remains the right
move for anything an answer cannot clear. A configured `[[channels]]` adapter
also pushes each freshly parked question — a toast naming the `trellis inbox
answer` command, one tick late at worst — so the loop's human half hears
about it without watching the board (decision 0043).

**Acting-role attribution.** `/trellis:act` records the acting role in
`.trellis/acting-role` at the root (ephemeral, gitignored) and removes it on
completion. The gate uses it to distinguish a mandated generator refreshing a
`generated` view from a hand-edit.

**Known limits of this binding:**

- Mandate authority is enforced by prompt with a deterministic backstop, not by
  compiled permission profiles.
- The gate sees Write/Edit, not shell-mediated writes.
- A crashed session can leave a stale `.trellis/acting-role`; the runtime
  reconciles its own lines at startup, and the marker carries a timestamp so
  the steward's lint can flag what remains.
- The wall clock is whatever invokes `trellis rituals`. A machine that sleeps
  misses nothing by itself — the next invocation fires each overdue row once,
  and `catchup = "skip"` records the window instead for a domain that would
  rather wait. Nothing fires once per missed window. The dispatcher has no
  such exposure: its loop reacts to the tree, not the calendar.
- The serving surface and the session logs are single-machine: two operators
  see two boards over one shared repository, and neither log is the ledger.
- The API carries no authentication. It binds loopback by default; binding
  anything else publishes an unauthenticated read of the whole domain. The
  session token in `/mcp/{token}` is attribution, not authentication: it says
  which session is calling so a question can name its plan, and it is not a
  secret.
- The ask channel shortens the escalation loop; it does not close it. A question
  nobody answers ends in the session deciding for itself. `--once` and
  `--no-http` have no channel at all, so a scheduled pass run that way is
  exactly as blind as it was before.
- Progress is what a session chooses to report, not an observed event stream, so
  a session that reports nothing looks idle. Parsing the harness's own
  structured output would fix that for one harness at a time; deferred
  deliberately (decision 0041).
- There is no event-driven plane at all (the `ingress` row above). A domain
  whose work arrives as outside events has to bring them in by hand.
- An escalation record notifies nobody *unless a channel is configured*: it
  satisfies the `escalation` service's audit-trail half on its own (decision
  0036), and recipients read the root, a generated view over open records, the
  daemon's board and API, or the report of the session that raised it. A push
  rides *over* the records through the channel-adapter seam in `trellis serve`
  (decision 0038); the adapter that ships is `[[channels]] kind = "herdr"`
  (decision 0043), and the record stays the system of record either way.
- Under `backend = "herdr"` the per-session budget is unenforced —
  `--max-budget-usd` works only with `--print` — so the complexity map's budget
  leg is refused in those templates rather than silently ignored, and the bound
  on a runaway session is the operator's attention. Completion is a settled
  state, not an exit code: a session that settles having done nothing reads as
  finished cleanly, and the plan's own status remains the truth that matters.
- Herdr's `blocked` state is heuristic screen-matching unless herdr's own
  claude integration hook is installed (`herdr integration install claude`),
  and it is session-level only: it toasts and holds the slot, and never writes
  a plan-level escalation record — only the session, under its mandate, does.
- The herdr socket protocol is pinned (19, pre-1.0). Drift refuses at startup
  by the ping gate, naming both numbers; there is no compatibility range until
  the pin measurably costs one (decision 0043).
- An advisory role's finding is durable only once its owner transcribes it, so a
  headless ritual whose findings nobody transcribes leaves them in a session
  log. Accepted with the ownership boundary it buys; a per-role escalation inbox
  is the pull-triggered fix. `trellis_progress` surfaces a finding while the
  session runs, which makes it *visible* — durability is still the owner's
  transcription, and no channel changes that.

## Staging — extracted by pull, not built ahead

- **Stage 2 — deterministic kernel (the CLI chartered in the steward mandate).**
  Pull-trigger: a check that must never be probabilistic (lint in CI), or a
  second binding worth having. Contents: deterministic lint, `mandate.md` →
  permission-profile compilation, `rituals.md` → cron generation, CODEOWNERS
  generation. The kernel is what makes pi/Codex/OpenCode bindings cheap.
  **Landed** as `cli/` (`trellis`, decision 0037): mechanical lint with the
  judgment remainder reported, dispatch scan, gate, lifecycle porcelain,
  escalation records, the five generated views, readiness's mechanical share,
  and scaffold. Cron generation shipped and was then removed with the forge
  clock it served (0039): the daemon reads cadences live, so there is no cron
  to generate or keep in step. Still open from the charter: `mandate.md` →
  permission-profile compilation.
- **Stage 3 — local runtime daemon (`trellis serve`, decision 0038).**
  **Landed**, and not as this entry staged it. The entry promised a thin
  ingress-only dispatcher on the Claude Agent SDK, pulled by an event the forge
  could not relay or by conversational latency. That pull never fired. What
  fired was different: local-first operation, where planes 2 and 3 had no
  binding at all without a forge; 0036's chartered push seam with no substrate
  to attach to; and 0037 collapsing the daemon's cost from a build to a loop
  over the existing kernel. So serve owns the clock (rituals and dispatch), a
  read-only serving surface (computed facts and a plan board), and the
  escalation channel-adapter seam — and still never the interactive plane.
  Ingress, the one service this stage originally promised, is now unbound
  outright: 0039 retired the forge relay it was deferred to, on the evidence
  that nobody used it. The staging discipline is worth keeping only if a stage
  can be recorded as wrongly predicted rather than retrofitted; this one was.
- **Unbound, waiting for a real event: `ingress`.** Pull-trigger: an outside
  event that actually arrives and that a human bringing it into a session
  handles too slowly. Whatever binds it must carry a mandate — the reason the
  read-only serving surface is not quietly widened into one.
- **Refused with a pull-trigger: a client-to-agent protocol (ACP) as the session
  transport (decision 0041).** Evaluated and declined — Claude is
  adapter-mediated, so the reference binding would gain an undeclared Node
  dependency; ACP has no budget expression and agent-defined model/effort
  options, so 0032's complexity map gets harder rather than portable. What
  shipped instead is a channel *back* from the session over MCP, which every
  harness speaks natively. Pull-trigger for revisiting: Claude Code speaking ACP
  natively, **or** a second harness adopted and blocked by the placeholder set's
  Claude Code vocabulary, **or** `mandate.md` → permission-profile compilation
  being built. Trellis as an ACP *agent* — `act` exposed to editors as
  `available_commands` — is a separate and still-open question about the
  `session` service, not about the spawn.
- **Rejected until evidence demands otherwise:** a hosted cloud runtime
  (premise 5: infrastructure ahead of evidence); push transports beyond the
  adapter seam; any write path on the serving surface *beyond* the single answer
  route 0041 buys — which relays a reply to a session already running under a
  mandate and touches no artifact — the errand request 0048/0051 buys, which
  names a plan already in the domain (0029) to the dispatch loop, the only
  spawner, and serve merely relays, and the status flip 0049 buys, which is
  the operator's own lifecycle verb behind the same relay, gates included.
