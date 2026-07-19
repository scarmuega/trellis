---
provenance: authored
status: accepted
date: 2026-07-19
---
# 0024 — Automation shapes: the selection rubric is codified, not left latent

## Context
The repo has chosen a member shape four times — the steward as a plugin agent
(0011), commands and hooks as the binding's adapter surfaces (0019), plan
authoring as a command with skill, agent, and hook each rejected (0020), skills'
two homes (0021) — yet the rubric behind those choices was never stated. A
contributor facing the next automation had to reverse-engineer it from
alternatives-rejected sections, and the naming rule (0010: bare verb, topic,
role) was silently doing selection work it never claimed. The natural home,
`spec/patterns.md`, is charter-bound to harness-neutral content inside the
structure; a Claude-vocabulary rubric there would break the axis 0022 drew
("runtime is execution/binding, patterns are content placement").

## Decision
Split the rubric along the contract/binding seam and state each half once:

- `spec/patterns.md` gains an **Automation shapes** pattern — harness-neutral
  classification by who triggers the automation and what relation it bears to
  judgment: a deliberate action is an `act` invocation (its spelling is the
  binding's adapter surface); recognized-situation know-how is a skill, placed
  per Cross-cutting procedures; a worker with identity and bounded authority is
  a role (an agent is a holder form, not a shape); a standing cadence is a
  `rituals.md` row; a no-judgment invariant belongs to the gate.
- The README's "Extending the plugin" promotes the bare-name rule into the
  selection rule for plugin contributors — verb → command, topic → skill,
  role → agent, no-judgment guard → hook — pointing at 0020 as the worked
  example.

Harness-specific facts (subagents cannot interact mid-flight; skills cannot
observe harness state as a trigger) stay stated once, in the decision log.

## Consequences
"Which shape is this automation?" has a stated answer for both audiences —
domain operators through the pattern, plugin contributors through the README —
without duplicating `runtime.md`'s reference-binding table or importing harness
vocabulary into `patterns.md`. Non-normative filing only; the spec stays v11.

## Alternatives rejected
State the rubric in `runtime.md` — restates its reference-binding table's
outcomes and the new pattern; 0022's wrong-axis argument cuts both ways. A
Claude-vocabulary rubric in `patterns.md` — breaks that document's harness
neutrality and its "inside the structure" charter. Leave the rubric latent in
the decision log — derivable is not stated: decisions record why past choices
were made, not how to make the next one.
