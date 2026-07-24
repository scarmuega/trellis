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
binding**: Claude Code plus a git forge.

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
lock, and a session that dies before claiming simply gets retried next tick.

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
- which enforcement is deterministic (hooks, branch protection, CI) and which is
  prompt-borne — stated honestly,
- where credentials live (never in the root — secrets policy),
- how actions are attributed in the ledger.

An instance records its binding choices in `conventions.md` under "Runtime
binding".

## Reference binding: Claude Code + git forge

| service | binding |
|---|---|
| session | Claude Code at the domain root with the `trellis` plugin; plan authoring rides the harness's plan mode via `/trellis:plan` (`commands/plan.md`) and persists to `plans/`; plan-effectiveness review via `/trellis:focus` (`commands/focus.md`) |
| act | `/trellis:act <role> [input]` (`commands/act.md`); headless: `claude -p "/trellis:act …" --plugin-dir <trellis checkout>` |
| schedule | forge cron (reference: `template/.github/workflows/rituals.yml`) → headless `/trellis:ritual <name>` |
| ingress | issues labeled `role:{name}` (reference: `template/.github/workflows/ingress.yml`) → headless act; relay non-forge events (email, tickets) into issues to keep one ingress and one ledger |
| gate | plugin hooks (`hooks/hooks.json` → `hooks/gate.mjs`): deterministic guards on Write/Edit — no hand-edits to `provenance: generated`, no edits to committed accepted decisions, frontmatter warning on new artifacts — plus branch protection + generated CODEOWNERS for core-class review |
| escalation | forge issues assigned to the `escalate-to:` role's holder (human `ref.md` should carry the forge handle); approvals are PRs |
| ledger | git history + forge threads; the acting role is recorded in the session marker and named in commits/comments |

**Plan dispatch.** `.github/workflows/dispatch.yml` — a cron on its own cadence
(daily by default, distinct from `rituals.yml`) scans `plans/` for `status:
ready` and fires one headless `/trellis:act <owner> advance …` per plan, modeled
on the ingress workflow's one-act-per-item shape for clean per-owner attribution.
The `plan dispatch` row in `rituals.md` records the standing behavior; the
steward is its scheduled-plane operator of record, but because the scan carries
no judgment it is implemented as this wiring, not a steward session (the steward
mandate's own "extract the deterministic parts into tooling"). If daily latency
proves too slow, a more responsive dispatcher is Stage-3 territory (below).

**Acting-role attribution.** `/trellis:act` records the acting role in
`.trellis/acting-role` at the root (ephemeral, gitignored) and removes it on
completion. The gate uses it to distinguish a mandated generator refreshing a
`generated` view from a hand-edit.

**Known limits of this binding** (accepted in decision 0019, resolved by
stage 2/3):

- Mandate authority is enforced by prompt with a deterministic backstop, not by
  compiled permission profiles.
- The gate sees Write/Edit, not shell-mediated writes.
- A crashed session can leave a stale `.trellis/acting-role`; the marker carries
  a timestamp so the steward's lint can flag it.

## Staging — extracted by pull, not built ahead

- **Stage 2 — deterministic kernel (the CLI chartered in the steward mandate).**
  Pull-trigger: a check that must never be probabilistic (lint in CI), or a
  second binding worth having. Contents: deterministic lint, `mandate.md` →
  permission-profile compilation, `rituals.md` → cron generation, CODEOWNERS
  generation. The kernel is what makes pi/Codex/OpenCode bindings cheap.
- **Stage 3 — dispatcher daemon (Claude Agent SDK).** Pull-trigger: an ingress
  event that cannot be relayed through the forge, or needs conversational
  latency (live support). A thin always-on process owning only ingress — never
  the interactive plane.
- **Rejected until evidence demands otherwise:** a hosted cloud runtime
  (premise 5: infrastructure ahead of evidence).
