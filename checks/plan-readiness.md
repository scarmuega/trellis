# Plan-readiness checklist

Canonical definition of *ready* for a Trellis plan: is this specified enough for
a taker to execute it without its owner in the loop? Referenced by
`/trellis:plan` (the owner's release offer) and the `trellis:coder` agent (the
taker's pickup gate) — this file is the single source; keep both pointing here
rather than copying the list. One bar, checked twice: what the owner releases is
what the taker accepts, so a plan never leaves `draft` on a bar the taker then
fails it on.

Items 1–8 apply to any taker, human or agent. Items 9–11 apply when the taker is
an agent working without a human present — the dispatch case (spec companion
`runtime.md`, "Plan dispatch"). A plan that passes 1–8 and fails 9–11 is
releasable to a human holder and not to an agent one; say which when you report.

## Any taker

1. **Objective, not a task list.** One statement of the outcome, in the domain's
   own vocabulary (`glossary.md`), separable from the approach. A plan whose
   objective *is* its steps cannot tell a taker when to stop.
2. **A done criterion the taker can evaluate itself.** "Improve onboarding" is
   not one; "signup issues a magic link and the flow is covered end-to-end" is.
   If checking done requires asking the owner, the plan is not ready.
3. **Refs resolve** (conventions-lint item 4): every `subdomains:`, `contexts:`,
   `metrics:`, and `decisions:` entry points at an existing file or anchor. An
   unresolvable `contexts:` ref means the taker does not know where the work
   lands.
4. **The change mechanics are determinable.** The plan's subdomains carry
   `induced-by:` edges, so the effective automation class is computable before
   work starts — the taker must know whether a change lands or is proposed
   (`core` never lands) rather than discovering it at delivery.
5. **The owner is real and covers it.** `owner:` names a role under `org/` whose
   mandate `scope:` covers the plan's subdomains. Releasing to a role whose
   mandate does not reach the work dispatches it into authority it lacks.
6. **Required authority is inside the mandate, or named.** Spend, publishing,
   and approvals the plan needs fit the owner's `authority:` — or the plan says
   explicitly which ones are escalations. Authority discovered mid-flight is a
   blocker by construction.
7. **No unresolved deferrals.** No "TBD", "decide later", "pick one of", open
   question, or unfilled placeholder in the body. A deferral inside a released
   plan is a question mailed to someone who cannot answer it.
8. **Decisions it rests on are recorded.** Choices the work depends on resolve to
   `decisions/` refs or are stated in the body. A taker must never have to
   re-derive a decision the domain already made elsewhere.

## Agent taker, no human present

9. **Underdetermination test.** Where the plan admits more than one
   implementation, the alternatives may differ in how the work reads — never in
   what the business gets. If two reasonable readings would deliver the business
   different things, that choice is the owner's and the plan is not ready. This
   is the judgment item; the rest are checks.
10. **The target is reachable and writable.** The `contexts:` name material the
    taker can change in the working tree it is given. Material the instance's
    boundary guarantees hold read-only (imported, vendored, external) is not
    implementable here — the plan owes the ref and the route instead.
11. **Verification exists.** The taker can check its own work: a test suite, a
    `contracts/` artifact, an eval, or a manual check the plan states. Without
    one, "done" is self-reported, which is the failure mode decision 0030
    rejected for progress. An interface-shaped done criterion — output another
    context, party, or system will consume — is verified by its contract: an
    existing one in the publishing context's `contracts/`, or one this plan
    hatches. Tests on one side of a seam do not verify the seam; an interface
    deliverable with no contract is verification that does not exist.

## What a failure means

**At release** (the owner, interactively): the plan stays `draft`. Nothing
reviews a draft and nothing dispatches it, so the cost of a failed item is a
revision, not a stalled queue.

**At pickup** (the taker): never guess, never narrow the plan to the part that is
specified. Flip it `ready → blocked`, escalate to the plan's owner naming each
failed item and the question it asks, and stop. The escalation is an escalation
record in the plan's own `## Escalations` section when the taker acts as the
plan's owner, and a report line for the owner to transcribe when it does not —
either way the plan states its own blocker, which is what lint item 24 checks.
The plan leaves the dispatch queue rather than being re-picked every tick, and
the escalation is the only work product — an unreleased plan is cheaper than a
wrong implementation of it.
