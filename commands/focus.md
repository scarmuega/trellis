---
description: Evaluate whether the current plans are effective — solution against problem, metrics as evidence
argument-hint: [scope — a plan, subdomain, strategy, or metric]
---

Evaluate the domain's plans against the problem they answer to, with metrics as
evidence: surface candidates (high-ROI moves, solution gaps), blockers, and
risks, and challenge plans whose value has quietly expired. Read-only analysis
plus conversation on the interactive plane of the runtime contract
(`${CLAUDE_PLUGIN_ROOT}/spec/runtime.md`); the scheduled twin is the `focus`
ritual (`org/focus` → the `trellis:focus` agent). Both walk the same canonical
checklist: `${CLAUDE_PLUGIN_ROOT}/checks/plan-effectiveness.md`.

Arguments: `$ARGUMENTS` — an optional scope, free text. Empty means the whole
root: every `active` and `blocked` plan plus the full coverage walk.

## Procedure

1. **Bind to the domain root**: the nearest directory at or above the working
   directory holding `conventions.md`, `problem/`, `solution/`, and `org/`. Not
   in a Trellis root → say so and stop. Read the root's `conventions.md`
   (authoritative over this command where they differ) — especially its plan
   schema and "Runtime binding" section (the escalation channel).

2. **Fix the scope**: match `$ARGUMENTS` against plan slugs, `problem/`
   subdomains, `strategy/` files, and metric anchors. Ambiguous → ask, showing
   the matches. Empty → the whole root. A scoped run walks only the checklist
   items touching the scoped artifacts and their edges.

3. **Load the evaluation base** (read-only): `market.md` need anchors;
   strategies at every maturity stage past `raw` — the stage sets the
   expectation (Strategy maturity pattern,
   `${CLAUDE_PLUGIN_ROOT}/spec/patterns.md`); the committed band and its
   `core-ranking` drive the coverage walk; the `induced-by:` edges on
   subdomains (effective automation policy); plans with status `active` or
   `blocked` — nothing evaluates a draft; `metrics/definitions.md` targets and
   `metrics/actuals/` within the freshness window from `rituals.md` (spec
   rule 5 — stale actuals become a finding, never evidence); git history for
   status ages.

4. **Work the checklist** item by item within scope, recording each finding
   with its full record — kind, item, refs, evidence, action, addressed-to
   owner — as the checklist specifies.

5. **Present and challenge**: findings ordered by attention rank — coverage of
   the top core subdomain first — each anchored in evidence, never in vibes.
   This is a conversation: the human sharpens, dismisses, or adds; a dismissed
   finding dies here and leaves no trace. No human present (headless run) →
   skip the conversation and carry every finding to the report.

6. **Offer follow-through** — each an explicit act, none automatic:
   - an accepted candidate → hand off to `/trellis:plan {topic}`; it graduates
     to a draft plan under its owner's hand (there is no candidate artifact);
   - a finding addressed to an owner not present → an escalation through the
     instance's escalation channel (default: a forge issue via
     `gh issue create`, one per finding, the record in the body);
   - a status flip (activate, block, retire) with the plan's owner present →
     the owner's authored edit, made at their direction.

   The analysis itself writes nothing.

7. **Report**: the scope walked, plans evaluated, findings by kind with their
   records (fenced `yaml`, one per finding), dispositions from the
   conversation, hand-offs made, escalations opened (refs), and any staleness
   that blocked judgment.
