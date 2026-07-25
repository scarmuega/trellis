---
provenance: authored
status: accepted
date: 2026-07-25
---
# 0031 — The coder: implementation as a portable holder, PR as its unit

## Context
0029 built a work queue — a plan the owner releases (`status: ready`), a
deterministic scan, `act(plan.owner)` per plan — and the holder branch routes an
agent-held owner to autonomous execution. But the plugin ships no agent that
*produces*. The steward enforces form and the focus challenges worth; both are
explicitly write-nothing or write-generated-only. A domain whose plan lands in
code has an empty queue-consumer: dispatch fires `act`, a generic session picks
it up with no gate on whether the plan is implementable, no rule about where the
code lands, and no convention for what happens to the work it could not finish.

Three failure modes make that gap concrete. An underspecified plan gets an
agent's plausible guess instead of a human's answer, and the guess is expensive
precisely because it looks like delivery. A partially finished implementation has
nowhere to sit — either it is landed as if complete or it evaporates with the
session. And the residue every implementation produces (deferred hardening, the
adjacent problem the code exposed) has no home, so it is either smuggled into the
same change or lost.

## Decision
Ship **`agents/coder.md`** (`trellis:coder`): a portable implementation holder,
the same seam as 0011 — identity is portable, authority is local. It binds to the
root that invoked it, reads the mandate of the role it acts as (under dispatch,
the plan's `owner:` — never a role named `coder`), and works within it.

Four behaviors define it:

- **A readiness gate, checked before any code.** The coder walks
  `checks/plan-readiness.md` and, on any failure, flips the plan `ready →
  blocked`, escalates the failed items as questions to the plan's owner, and
  stops. Blocking (not leaving it `ready`) is what removes it from the dispatch
  queue instead of re-burning a session every tick; `blocked` dwell is already
  caught by plan-effectiveness item 9, so the stall stays visible.
- **Run to completion or to a blocker.** Claim by flipping `ready → active`,
  implement within the plan's `contexts:` under the effective automation class,
  verify against the context's own tests and contracts, and escalate — never work
  around — anything needing a human.
- **The PR is the unit of delivery.** Any code change ends as a pull request:
  draft while the implementation is unfinished, ready for review when the done
  criterion is met and checks pass. The coder never merges its own PR — which
  changes may land without review is the automation class expressed in the
  binding, not the agent's call.
- **Residue becomes a `draft` plan, never a `ready` one.** Leftover work is filed
  as one new plan the owner must release. An agent that could enqueue its own
  next job would close a loop with no human in it.

**A new checklist asset, `checks/plan-readiness.md`**, holds the definition of
ready — items 1–8 for any taker, 9–11 for an agent taker with no human present.
It is shared, per the 0011 precedent that lifted the lint list out of two
near-duplicates: `/trellis:plan`'s release offer checks it and the coder
re-checks it at pickup, so a plan never leaves `draft` on a bar the taker then
fails it on.

**No template role.** The steward and the focus ship in `template/org/` because
they operate the *model* — every domain has conventions to enforce and plans to
challenge. An implementation role operates the *business*: it is contingent (a
bakery domain has no codebase), its `scope:` is bounded-context-specific, and a
domain with several code-bearing contexts wants several such roles under its own
names. The template ships no domain roles, and this is one. Adoption is a
procedure in the conventions skill instead: write the mandate, point
`holder/ref.md` at `trellis:coder`, name that role as the `owner:` of
code-bearing plans — dispatch then routes to it with no further wiring.

Non-normative throughout: spec stays **v14**, no schema change, no new status, no
new lint item, no new plane or contract service. The coder is a holder form, per
the Automation shapes pattern.

## Consequences
Plan dispatch has a producer, so the queue 0029 built does work end to end for
code-bearing plans, and the three failure modes get structural answers: an
underspecified plan comes back as a question, an unfinished one as a draft PR, a
residual one as a draft plan. `active` keeps meaning "in flight" — the coder
never retires a plan, because a merged PR is the completion event and the verdict
is the owner's (0030's owed-verdict point), which leaves finished-but-unmerged
work visible as active dwell rather than as a silent closure.

The PR-always rule is slightly stricter than the runtime's automation table,
where `generic` reads "agent commits autonomously." It is satisfied through a PR
the binding may auto-merge: the class still decides *who lands it*, and the
domain gains one uniform ledger entry per implementation. `core` is unchanged —
the coder structurally cannot land it.

No eval suite ships with it. Lint item 6 exempts `ref.md` holders, and a coder
fixture needs a seeded codebase plus a forge, which the current harness
(`evals/`) does not model — the honest statement is that this agent's behavior is
prompt-borne and unverified by tests today. The readiness gate being a file
rather than prompt prose is what makes that testable later, and fixable now.

## Alternatives rejected
**A `/trellis:code` command as the interactive twin** (the 0025 command-and-role
pair) — focus earned its pair because the checklist is the value and a human
wants to walk it in conversation; implementation interactively is just the
harness doing what it already does, and a command would wrap plain coding in
ceremony. **A template `org/coder/` role** — ships a dead role into every
non-software domain and pre-names a role whose scope only the instance knows.
**A `pr:` field or an `implemented` status on the plan schema** — a normative
change to carry what the PR body and git already hold; 0029 remains the standard
for a new status (it drives dispatch). **Letting the coder merge `generic`-class
PRs** — a second authority path that diverges from the branch protection the
binding already expresses, for no gain over auto-merge. **Leaving an
underspecified plan `ready` and just escalating** — re-dispatches it every tick,
which is the stalled queue item 18 exists to catch, paid for in sessions.
**Letting the coder release its follow-up as `ready`** — an agent queueing its
own next job, with no human between one implementation and the next.
