---
provenance: authored
status: accepted
date: 2026-07-24
---
# 0029 — Plan readiness and dispatch: a `ready` status the runtime advances

## Context
A plan had no way to signal "specified enough for an agent to take over and run."
Its `status:` — `draft | active | blocked | retired` — made `active` carry two
things at once: the owner has *committed* to executing it, and it is *in flight*.
Nothing let a runtime scrape `plans/`, select the ones ready for autonomous
pickup, and start work. The gap matters because agents changed the economics of
execution (the spec's own opening premise): a well-specified plan is now
handable to an agent, but only if the release is an explicit, scrapable signal.

The reduction that shapes the answer: `act` invokes a **role**, never a plan, and
every plan already carries `owner:` (an org role). So "an agent takes over a
plan" is `act(plan.owner, "advance plans/{slug}.md")` — the feature needs a
readiness signal, dispatch semantics over the existing primitive, and a binding,
not a new plane.

## Decision
Add a plan status **`ready`** between `draft` and `active`: the owner has
released a specified plan for a taker. The lifecycle is
`draft → ready → active → blocked → retired`. This is the whole normative change
(a schema enum value plus its lifecycle comment in `spec/model.md`); no Rule
references plan status, so none changes. Spec **v13 → v14, additive** — existing
plans stay valid.

**Dispatch is `schedule` + `act` composed** — the action-sibling of the `focus`
ritual (focus evaluates plans and escalates; dispatch advances them). On the
dispatch cadence the runtime enumerates `ready` plans (a deterministic scan) and
invokes `act(owner)` per plan. Two properties fall out for free:
- **Agent-vs-human routing reuses `owner`'s holder type** — the holder branch
  `act` already implements: an agent-package owner advances the plan
  autonomously; a human-held owner receives a handoff, never impersonated. No
  new `assignee` field.
- **Idempotency for free** — the taker flips `ready → active` as it claims the
  work, so a plan leaves the queue the moment work starts; ticks never
  double-dispatch, and a session that dies before claiming is simply retried.

The reference binding is a dedicated `dispatch.yml` cron (its own cadence,
distinct from `rituals.yml`) that scans and fires one `/trellis:act <owner>` per
plan, modeled on `ingress.yml`'s one-act-per-item shape for clean per-owner
attribution. A `plan dispatch` row in `rituals.md` records the standing behavior
with **`org/steward`** as scheduled-plane operator of record; because the scan
carries no judgment it is implemented as this wiring, not a steward session —
the steward mandate's own "extract the deterministic parts into tooling." The
steward gains no spend, publish, or approve: it only starts the owner's act.

Two checks keep it honest: `checks/conventions-lint.md` item 4 enumerates the
legal plan statuses (validating `ready`); `checks/plan-effectiveness.md` counts a
`ready` plan as advancing its strategy in the coverage walk (so a queued plan
isn't a false `gap`) and adds item 18 — a `ready` plan the dispatcher never
drains within a cadence is a stalled queue (`blocker`).

## Consequences
`active` now means strictly "in flight," and the runtime has a plan-driven
work queue with no new plane, contract service, or infrastructure. Focus
coverage credits `ready` plans but its per-plan metric/challenge items skip them
(no measurement horizon yet). The eval fixtures stay green — they contain no
`ready` plans, so their expected findings are unchanged. A more responsive
dispatcher than a daily cron remains **Stage 3** (the dispatcher daemon), pulled
only if daily latency proves too slow — the forge cron suffices until then
(premise 5).

## Alternatives rejected
**Derive readiness** (`active` + owner-holder-is-agent + not-blocked, no new
field) — most derivation-pure, but gives the owner no explicit opt-in and leaves
idempotency to an external lock or a git-history movement check, weak in a
stateless git runtime. **A dedicated `dispatch: ready` field** orthogonal to
status — duplicates what status + owner-holder already say. **A separate
`assignee` ref** on the plan — a second identity edge that drifts from `owner`
and its authority. **A new trigger plane or contract service** — dispatch
reduces to `schedule` + `act`; a fourth plane would claim novelty it doesn't
have (the plan is already *in* the domain, not entering it). **The Stage-3
dispatcher daemon now** — infrastructure ahead of evidence; the scheduled scan
the forge already provides covers the need.
