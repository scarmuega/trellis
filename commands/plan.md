---
description: Draft a Trellis plan through the harness's plan mode and persist it to plans/
argument-hint: [topic of the plan]
---

Create one Trellis plan — a business execution artifact (campaign, experiment,
initiative, journey), never a code plan — by riding the harness's plan mode:
read-only research, an approved draft, then a conventions-compliant artifact in
`plans/`. A session-plane authoring aid of the runtime contract
(`${CLAUDE_PLUGIN_ROOT}/spec/runtime.md`).

Arguments: `$ARGUMENTS` — the topic of the plan, free text. No topic means: list
the existing plans (name, status, type) and ask what to plan.

## Procedure

1. **Bind to the domain root**: the nearest directory at or above the working
   directory holding `conventions.md`, `problem/`, `solution/`, and `org/`. Not
   in a Trellis root → say so and stop. Read the root's `conventions.md`
   (authoritative over this command where they differ) — especially its plan
   schema, plan-type registry, and tag registry.

2. **Determine the type**: match the topic against the plan-type registry.
   Ambiguous → ask, showing the registry. Not yet registered → agree the name
   and a one-line definition with the user now; the registry edit lands in
   step 7 (types register before use, but writes happen only after approval).
   A topic that is really a program of independently workable moves → offer
   the Plan decomposition pattern (`${CLAUDE_PLUGIN_ROOT}/spec/patterns.md`):
   an umbrella plan plus sibling sub-plans (`plans/{parent}-{piece}.md`), each
   drafted through this command — never a `plans/{plan}/` folder.
   A topic advancing a strategy biases the type to its stage (Strategy
   maturity pattern, `${CLAUDE_PLUGIN_ROOT}/spec/patterns.md`): `defined`
   wants an `experiment` with an explicit decision criterion; `validated`, an
   `initiative`; `established`, hardening or optimization work — propose the
   stage-matched type first.

3. **Resolve the owner**: every plan carries `owner:`. List the roles under
   `org/` whose mandate `scope:` overlaps the plan's likely subdomains, propose
   the best fit, confirm with the user. No plausible owner → stop and suggest
   adding the role first.

4. **Enter plan mode**: already in plan mode → proceed. Otherwise request it
   (`EnterPlanMode`). If the harness offers no plan mode (headless run, another
   binding), emulate its discipline: read-only research, a visible draft, and
   the user's explicit approval before any write.

5. **Research the domain** (read-only): the subdomains the plan touches and
   their `induced-by:` edges (effective automation policy), the committed
   strategies behind them and the `funded-by:` edges naming what sustains
   those strategies, `solution/context-map.md` and the bounded contexts
   the plan executes through, the `metrics/definitions.md` anchors that will
   measure it, and prior `plans/` and `decisions/` that overlap. Use terms from
   `glossary.md`; surface conflicts with active plans.

6. **Draft for transposition**: build the plan in the harness plan file as
   usual, but shaped like the artifact: a frontmatter block naming the exact
   `type:`, `subdomains:`, `contexts:`, `metrics:`, `decisions:`, `tags:`, and
   `owner:` values — every ref resolved to an existing file or anchor while
   drafting, never after — then objective, approach, measurement, risks and
   escalation triggers. Present via `ExitPlanMode`; feedback → revise and
   re-present.

7. **Persist on approval**: first land any registry additions (plan type, new
   tags) in `conventions.md`; then write `plans/{slug}.md` — the slug per
   rule 10, a distinctive retrieval key naming the move, never a date or
   "q3-plan" — with frontmatter `provenance: authored`, the confirmed `owner:`,
   `status: draft`, the registered `type:`, and the resolved ref lists, then
   the body transposed from the draft. Offer to release it in the same breath —
   a deliberate owner's act that commits execution, not just content: flip to
   **`ready`** to hand it to the runtime's plan dispatch (an agent-held owner
   advances it autonomously, a human-held owner receives it as a handoff), or to
   **`active`** if the owner is driving it themselves now. The focus ritual
   evaluates `ready`, `active`, and `blocked` — never a draft. Do not commit;
   version-control mechanics follow the instance's runtime binding.

8. **Lint before reporting**: run the plan-relevant items of
   `${CLAUDE_PLUGIN_ROOT}/checks/conventions-lint.md` against the new artifact —
   frontmatter valid (item 1); status, registered type, resolving refs
   (item 4); glossary terms (item 7); registered tags (item 8). Fix failures
   before reporting.

9. **Report**: the artifact path, status, type, owner, the refs it carries, any
   registry additions to `conventions.md` (flag them to its owner when
   different), and — if left `draft` — the reminder that nothing reviews a
   draft: flip to `ready` to release it for agent dispatch, or `active` when the
   owner drives it.
