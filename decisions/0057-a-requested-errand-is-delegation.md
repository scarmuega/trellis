---
provenance: authored
status: accepted
date: 2026-08-15
---
# 0057 — A requested errand is delegation, not impersonation

## Context

Operator errands (0048, 0051) validate a request with the gate dispatch
uses: exists, not terminal, owned, **holder not a declared human**, not in
flight. Dogfooding hit the conflation on the first try: the founder — the
human holder of the role that owns most plans in a young root — asked the
board for an errand on their own plan and was refused with "human-held —
this is a handoff, not a session."

The never-impersonate rule (0029; spec/runtime.md) is right where it
applies: the runtime's own triggers — the dispatch scan on its tick, the
ritual cadence on its clock — must never spawn a headless session that acts
as a human with nobody present. The measured version of that argument is
0045's: a human-held owner used to buy a whole session whose entire work
product was reading everything and correctly stopping at act's
never-impersonate rule.

But the request plane's trigger *is* the human. The ask originates with the
operator, on a loopback surface whose trust model already lets that operator
flip plan statuses (0049) and feed free-text instructions into sessions on
agent-held plans (0048 admits the route is a prompt door and leans on the
loopback default). 0048 motivated errands with "a human at plane 1 can type
`/trellis:act <owner> …` today; what was missing is a way to request it
without opening a session by hand" — and the inherited gate defeats that
purpose precisely for the plans the operator most plainly has authority
over: their own. The refusal handed the work back to the person delegating
it.

## Decision

**A holder-originated request spawns; only runtime-originated triggers hand
off.** The never-impersonate rule is rescoped, not weakened. `kind: human`
still short-circuits the dispatch scan and the ritual cadence — no session
with nobody present, no change to a single scan path. An operator-requested
errand on a human-held owner runs as **delegated execution**: the session
acts under the role's mandate at the holder's direction. Asking an agent to
do work on your behalf is not the agent impersonating you; the request is
the explicit authority that spec/rationale.md item 10 requires.

This is not a new primitive. 0048 rejected the sibling shape and 0051
reduced every errand to a template over `act`; what changes is act's holder
branch, which gains a third disposition: a package acts, an agent ref is
adopted, a human ref hands off — *unless the invocation is an errand the
holder requested themselves*, in which case the session proceeds under the
mandate at their direction. Ownership never moves: the mandate bound is
still the plan's `owner:`, which is also what keeps the route non-ingress
(0029's reduction, restated in runtime.md's ingress row).

Three boundaries keep delegation from becoming abdication:

- **Automation classes are untouched.** A core-class change still lands as a
  proposal, so the holder reviews the delegated work; the review gate is
  where the human stays accountable.
- **The mandate's personal share stays personal.** A delegated session never
  exercises `authority: approve` on the holder's behalf; anything
  approval-shaped returns as a proposal or an escalation. The agent does
  work; judgment stays with the human.
- **The trail names the trigger.** The errand and its instruction are the
  attribution — commissioned work, never the runtime's initiative.

The runtime renders the fact rather than letting the session infer it
(0045's discipline): a `{delegation}` placeholder is set on every errand
fire — the empty string for an agent-held or undeclared holder, a sentence
naming the disposition for a declared-human one — and the default errand
templates carry it. An instance template that drops the placeholder owns the
variant, exactly as 0051 ruled for every other template edit.

Mechanically: `validate_errand` loses its `Handoff` refusal — the route and
the drain pass share the function, so both open at once — and the scan's
handoff path is untouched. The spec's flat sentence ("`human` means never
spawn a session for this role's work") is rescoped to the runtime's own
triggers in model.md and runtime.md, and act's step 4 gains the exception.
A spec change is a spec bump: v22, release 0.22.0, lockstep per 0056.

## Consequences

- The board's errand menu works on the operator's own plans; the founder can
  commission a refine without opening a session by hand.
- The loopback bind carries more weight: a local caller can now start a
  session under any mandate, human-held included. Stated honestly, this adds
  degree, not kind — the same caller could already request errands on
  agent-held plans, already flip statuses (0049), and already do the work at
  plane 1. Non-loopback binds were already declared load-bearing (0048); the
  config comment stands.
- Undeclared holders behave exactly as before — the change touches only the
  declared-`human` branch, so the pre-v19 compatibility promise (route, and
  let the session decide) is intact.
- An interactive `/trellis:refine` or `/trellis:act` on a human-held role is
  the same disposition — the human in the session is the holder-originated
  trigger — so `commands/refine.md`'s handoff sentence narrows with the
  spec.
- spec/model.md, spec/runtime.md, commands/act.md, commands/refine.md, the
  default templates, and the skeleton's founder ref amended in the same
  commit (the 0041 precedent).
