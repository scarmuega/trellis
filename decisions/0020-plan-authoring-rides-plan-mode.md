---
provenance: authored
status: accepted
date: 2026-07-12
---
# 0020 — Plan authoring rides the harness's plan mode

## Context
Plans are the domain's execution artifacts (`plans/{plan}.md`: lifecycle
`status:`, registered `type:`, refs to subdomains, contexts, metrics,
decisions). Drafting one well wants exactly the discipline Claude Code's plan
mode already provides: read-only research, an incrementally built draft, and
explicit approval before anything lands. But the harness's plan file is
harness-controlled — no setting redirects it into the project — and lives
outside any Trellis root, so nothing persists into `plans/` by itself.
Candidate member types for closing that gap: a slash command, a skill, an
agent, a hook.

## Decision
A slash command: `commands/plan.md` → `/trellis:plan` (bare verb per
decision 0010; the harness's own `/plan` prefix is a different, unnamespaced
token and stays untouched). The command enters or reuses plan mode, researches
the domain read-only, drafts *for transposition* — the exact frontmatter
values resolved while drafting — and on `ExitPlanMode` approval persists
`plans/{slug}.md` with full conventions frontmatter. All writes, including
registry additions (new plan types, new tags — registered before use), are
deferred to post-approval, reconciling "register before use" with plan mode's
no-writes discipline.

Authoring is direct on the interactive plane — no `act()` wrapper and no
`.trellis/acting-role` marker. The human is present and approval *is* the
review (`spec/runtime.md`, plane 1); the gate requires attribution only for
`provenance: generated` artifacts, and a plan is `authored`. Headless planes
that need to create plans keep going through `/trellis:act` as usual.

The artifact persists as `status: draft` with an immediate offer to flip
`active`: approval approves content, not commencement — activation commits
execution and is the owner's act. The command warns that the plan-review
ritual walks only `active` and `blocked`, so a draft is invisible to the
heartbeat until flipped.

Plan authoring is a binding convenience under the existing `session` contract
service; the seven-service runtime contract of decision 0019 is unchanged.

## Consequences
Creating a plan costs one command and one approval, and the result passes the
plan lint (checklist item 4) by construction — refs resolve during drafting,
never after. The command text is the enforcement for slug quality (rule 10)
and for the business-plan/code-plan distinction; both are prompt-borne, per
0019's honest split. Sessions without plan mode (headless, other bindings)
get an emulated discipline rather than a silent degradation.

## Alternatives rejected
A skill auto-triggering on "plan mode inside a Trellis root" — over-triggers
structurally, since most plan-mode sessions in a root are engineering plans,
not business plans, and a skill cannot observe plan-mode state as a trigger.
An agent — plan mode is an approval loop with the user, and subagents cannot
interact mid-flight. A hook on `ExitPlanMode` — the gate's charter is "guards
only what needs no judgment", and it would conscript every code plan ever
approved in a root. Redirecting the harness plan file into `plans/` — no such
setting exists, and the design would couple to harness internals instead of
riding the stable approval seam.
