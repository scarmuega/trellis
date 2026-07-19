---
provenance: authored
status: accepted
date: 2026-07-19
---
# 0025 — Focus: plan effectiveness as a command-and-role pair

## Context
The heartbeat checks form but never worth. The steward's three rituals ask "is
the domain convention-compliant"; the `plan review` row asked a human to walk
active plans weekly, with no stated method and no headless path (executor
`<owner>` — absent from the workflow's RITUALS list). Nothing evaluates the
solution against the problem with metrics as evidence: committed strategies no
plan advances, core attention misallocated against `core-ranking`, blocked
plans nobody re-litigates, plans structurally valid but value-dead. Premise 5
calls the framework an epistemic machine; the machine had no organ for asking
whether its experiments are working. The rubric (0024) supplies the shapes on
hand: command, skill, agent, hook, ritual row.

## Decision
One automation, two surfaces, shared substance. `/trellis:focus` is the
deliberate verb on the interactive plane — read-only evaluation plus
conversation, no plan mode (nothing drafts-to-persist); the `trellis:focus`
agent is the portable holder of a new template role `org/focus/` (mandate
local, identity portable — the 0011 seam, exercised a second time), executor of
a `focus` row that replaces `plan review` in the template's `rituals.md` and
joins the workflow's RITUALS default. Both surfaces walk one canonical
checklist, `checks/plan-effectiveness.md` (the 0011 lift applied at birth
instead of after duplication): coverage of the needs → strategy → subdomain →
plan chain, metric movement, attention allocation against `core-ranking`,
blockers and risks, and challenges to value-dead plans.

The mandate is advisory-only — spend none, publish none, approve nothing,
`scope: []` because focus changes no artifact, ever. It proposes; owners
decide. Findings are escalation-only (0016's shape) and carry a structured
record — kind (`candidate|gap|blocker|risk|challenge`), checklist item, refs,
evidence, action, addressed-to owner — which makes them dedupable across runs
and gradeable. An accepted candidate graduates to a `plans/{slug}.md` draft
through `/trellis:plan`, under its owner's hand.

The pair ships with evals (`evals/focus/`): seeded fixture domains, one per
evaluation frame plus a zero-finding precision guard, graded deterministically
on the finding records and on the never-writes boundary — "an eval is a
requirement on an agent made testable" (patterns.md), applied to the plugin's
own member at birth rather than after drift.

Naming: 0010 admits bare topic *or* role; `focus` is taken as both — one
retrieval key across command, agent, role, and ritual row (rule 10).

## Consequences
The heartbeat now audits worth as well as form, headlessly; the human plan walk
survives as the interactive surface, judgment intact — and a domain wanting a
separate human row can keep one, since `rituals.md` is instance-owned. Spec
stays v11: no new kind, no schema change; `runtime.md`'s session row and the
template's binding gain a line. `/trellis:ritual` generalizes past the steward,
which was the last only-agent assumption in the commands. 0020's mention of
"the plan-review ritual" describes a row this decision replaces; the decision
itself stands unedited (rule 6). The evals introduce the plugin's first
executable checks on its own prompt-borne members; grading is deterministic on
kind/item/refs and the no-writes boundary, while evidence and action quality
stay human-judged.

## Alternatives rejected
Extend the steward — collapses two mandates into one identity: the steward
checks form and must stay dull (its value is that it never judges content);
focus is nothing but judgment. One agent doing both invites the steward to
opine and focus to nitpick. A skill — a topic that loads on recognition has no
cadence and no authority; the scheduled plane needs a mandated executor, and
"challenge my plans" is a deliberate act, not a recognized situation. A command
only — strips the scheduled plane: a ritual is `act(executor, procedure)` and a
command is not an executor; the old row's `<owner>` executor was exactly this
gap. Keep plan review alongside focus — two weekly walks over the same plans is
the documentation ceremony the model exists to avoid. A persisted report
artifact — rule 7: a report with no lifecycle or owner of its own is a
grouping; 0016 set the precedent that sweeps are escalation-only, and the run
report plus one issue per finding already is the report. A `candidate` artifact
kind — a candidate nobody has committed to is exactly what `status: draft`
plans model; a second larval kind splits the plan lifecycle for no gain.
