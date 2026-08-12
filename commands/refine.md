---
description: Refine a plan's content as its owner — reshape scope, never execute it
argument-hint: <plan> [instruction]
---

Refine one plan's *content* — simplify its scope, split it, refactor its
structure — as an act of the role that owns it. Refine is
`act(owner, refinement)`: this command resolves the plan's `owner:` and
delegates to the `/trellis:act` procedure with the refinement contract below as
the input. It never executes the plan; advancing a plan toward its objective is
dispatch's errand, not this one. Runtime contract:
`${CLAUDE_PLUGIN_ROOT}/spec/runtime.md`.

Arguments: `$ARGUMENTS` — the first token is the plan (`plans/x.md`, `plans/x`,
or `x`); everything after it is the instruction. No instruction means: report
the plan's refinement candidates (scope seams, natural split points, stale
sections, readiness gaps a rewrite would close) and await one.

## Procedure

1. **Bind to the domain root** (nearest directory at or above the working
   directory holding `conventions.md`, `problem/`, `solution/`, and `org/`; not
   found → say so and stop). Read the root's `conventions.md` — authoritative
   over this command where they differ.

2. **Resolve the plan**: normalize the argument to `plans/{name}.md` and read
   it. Not found (live or under `archive/`) → say so and stop. `status:
   retired` or archived → refuse: the terminal tier is frozen content, and a
   reshape of finished work is a new plan, not an edit. No `owner:` → say so
   and stop; ownerless plans are a lint finding, not a refinement target.

3. **Delegate to act**: perform the `/trellis:act` procedure with the plan's
   `owner:` as the role and the refinement contract as the input — resolve the
   mandate, record the acting role in `.trellis/acting-role` (under dispatch
   the runtime owns this marker, decision 0045), adopt the holder, execute
   within authority, remove the marker when done. A holder declaring a human
   becomes a handoff, never an impersonation, per act's own procedure.

4. **The refinement contract** — the input act executes:
   - The write target is the plan artifact itself, plus any new plans a split
     produces. Nothing else: never claim the plan, never advance its
     `contexts:`, never touch the solution or the problem space.
   - Apply the instruction to the plan's content: tighten the objective, cut
     or split scope, restructure the approach, repair its readiness gaps
     (`checks/plan-readiness.md` is the bar its release will be judged by).
   - New plans from a split are filed `status: draft` with this plan's
     `owner:`, and sequencing between the pieces is declared with `awaits:`.
   - Keep frontmatter valid per the conventions; preserve `status:` unless the
     instruction says otherwise.
   - The plan's automation class still decides how the edit lands (act step
     5); anything the instruction requires beyond the owner's authority is an
     escalation, not an improvisation.

5. **Report**: the plan refined, the instruction, what changed (with commits
   or PR refs per the automation class), plans created by a split, and
   escalations raised — quoting each record and where it landed.
