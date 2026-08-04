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
advance. On the dispatch cadence the scheduled plane **enumerates the `ready`
plans** — a deterministic scan, no judgment — and for each invokes
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

## Reference binding: Claude Code + `trellis serve`

The harness runs the sessions, the daemon owns the clock, and a git forge
carries review. The forge is not the runtime: it holds the ledger and the
approval gate, and nothing schedules or triggers from it (decision 0039).

| service | binding |
|---|---|
| session | Claude Code at the domain root with the `trellis` plugin; plan authoring rides the harness's plan mode via `/trellis:plan` (`commands/plan.md`) and persists to `plans/`; plan-effectiveness review via `/trellis:focus` (`commands/focus.md`) |
| act | `/trellis:act <role> [input]` (`commands/act.md`); headless: `claude -p "/trellis:act …"`, which is the default argv template `trellis serve` spawns — against an *installed* plugin, since the daemon runs where the operator's harness already has one. A domain operating from an uninstalled checkout adds `--plugin-dir` to the template in `runtime.toml` |
| schedule | `trellis serve` — an in-process timer over `rituals.md` rows, read at every pass, spawning one headless `/trellis:ritual <name>` per due row |
| ingress | **unbound.** No event-driven plane ships: an outside event reaches the domain when a human brings it into a session. The daemon's HTTP surface is read-only by construction and is deliberately not a trigger door — a call that could invoke a role would be a plane with no mandate behind it. An instance that needs one binds it and records the choice |
| gate | plugin hooks (`hooks/hooks.json` → `trellis gate`, falling back to `hooks/gate.mjs` where the binary is absent): deterministic guards on Write/Edit — no hand-edits to `provenance: generated`, no edits to committed accepted decisions, frontmatter warning on new artifacts — plus branch protection + generated CODEOWNERS for core-class review |
| escalation | escalation records in the root: a fenced `yaml` block under `## Escalations` in the artifact the escalation concerns, written by that artifact's owner (schema in `template/conventions.md`); read from the daemon's board and `/api/escalations`, or the root itself; approvals are PRs |
| ledger | git history; the acting role is recorded in the session marker and named in commits. Per-session logs under `.trellis/runtime/logs/` are the daemon's trail — gitignored, single-machine, never the ledger |

**Plan dispatch.** The daemon runs the deterministic scan on its own cadence
(daily by default, read from the `plan dispatch` row in `rituals.md`) and starts
one headless `/trellis:act <owner> advance …` per `status: ready` plan —
one act per plan, for clean per-owner attribution. The steward is the row's
operator of record, but because the scan carries no judgment it is implemented
as this wiring, not a steward session (the steward mandate's own "extract the
deterministic parts into tooling"). It maps the plan's `complexity:` tier to the
session's model, effort, and budget: the tier's meaning is the spec's, the
effort levels, model names, and prices are this binding's — retuned in
`runtime.toml`, never in a plan. It reads `awaits:` with the same
frontmatter-only extraction and holds any `ready` plan whose targets are not all
`retired` — one `status:` lookup per target, still no judgment. A concurrency
cap bounds how many sessions run at once, never how much work a cadence may
start: a scan that could not start everything it found is not recorded as run,
so the overflow starts as slots free rather than waiting a full cadence.

**The serving surface.** The daemon serves the tree read-only over HTTP — the
computed facts the commands print, the generated views, the open escalation
records, and an embedded plan board over them. Read-only is structural: there is
no write path, so every change still enters through a session bound by its
mandate, the gate, and the artifact's automation class. It binds loopback by
default and carries no authentication.

**Implementation holders.** A plan whose `contexts:` land in code needs a holder
that can write it; the plugin ships `trellis:coder` as the portable one (decision
0031). A domain adopts it by creating an implementation role — a local mandate
scoping the subdomains and contexts it may change, plus a `holder/ref.md` to the
agent — and naming that role as the `owner:` of code-bearing plans; dispatch then
routes to it through the agent branch of `act`, with no extra wiring. The coder
gates on `checks/plan-readiness.md` and blocks rather than guesses, delivers code
as a PR (draft while unfinished) and never merges it, and files residue as a
`draft` plan. The class still decides who lands the change: `generic` and
`supporting` PRs may be auto-merged by this binding, `core` waits for its owner.

**Escalations.** This binding keeps escalations *in the tree*: the artifact that
carries the problem carries the escalation, as body content its owner authored
(decision 0036). A blocked plan therefore states its own blocker, so what stops a
loop is reconstructible from a checkout — the Loop observability pattern applied
to the escalation channel, where a forge thread is state filed outside the tree.
Because it is authored content, only the artifact's `owner:` writes a record: an
agent acting *as* that owner (the coder under dispatch) writes its own, while an
advisory role that owns nothing — `org/focus`, `org/steward` — reports its
findings for the owner to transcribe, which leaves both roles' write boundaries
exactly as they were. An escalation with no artifact to sit in stays in the
acting session's report.

**Acting-role attribution.** `/trellis:act` records the acting role in
`.trellis/acting-role` at the root (ephemeral, gitignored) and removes it on
completion. The gate uses it to distinguish a mandated generator refreshing a
`generated` view from a hand-edit.

**Known limits of this binding:**

- Mandate authority is enforced by prompt with a deterministic backstop, not by
  compiled permission profiles.
- The gate sees Write/Edit, not shell-mediated writes.
- A crashed session can leave a stale `.trellis/acting-role`; the marker carries
  a timestamp so the steward's lint can flag it.
- The clock runs where the daemon runs. A machine that sleeps or shuts down
  misses cadences; catch-up on waking fires each overdue row once, and
  `catchup = "skip"` records the window instead for a domain that would rather
  wait. Nothing fires once per missed window.
- The serving surface and the session logs are single-machine: two operators
  see two boards over one shared repository, and neither log is the ledger.
- The API carries no authentication. It binds loopback by default; binding
  anything else publishes an unauthenticated read of the whole domain.
- There is no event-driven plane at all (the `ingress` row above). A domain
  whose work arrives as outside events has to bring them in by hand.
- An escalation record notifies nobody: it satisfies the `escalation` service's
  audit-trail half and not its "channel they already watch" half (decision 0036).
  Recipients read the root, a generated view over open records, the daemon's
  board and API, or the report of the session that raised it.
  A binding that needs a push adds one *over* the records — the channel-adapter
  seam in `trellis serve` (decision 0038) is where it attaches, and the record
  stays the system of record.
- An advisory role's finding is durable only once its owner transcribes it, so a
  headless ritual whose findings nobody transcribes leaves them in a session
  log. Accepted with the ownership boundary it buys; a per-role escalation inbox
  is the pull-triggered fix.

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
- **Rejected until evidence demands otherwise:** a hosted cloud runtime
  (premise 5: infrastructure ahead of evidence); push transports beyond the
  adapter seam; any write path on the serving surface.
