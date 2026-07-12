---
description: Execute a ritual from rituals.md as its mandated executor role
argument-hint: <ritual name>
---

Execute one ritual from the domain's `rituals.md` — the scheduled plane of the
runtime contract (`${CLAUDE_PLUGIN_ROOT}/spec/runtime.md`). A ritual is
`act(executor, procedure)`: this command looks up the row and delegates to the
`/trellis:act` procedure.

Argument: `$ARGUMENTS` — the ritual name, matched case-insensitively against the
ritual column of `rituals.md`.

## Procedure

1. **Bind to the domain root** (nearest directory at or above the working
   directory holding `conventions.md`, `problem/`, `solution/`, and `org/`; not
   found → say so and stop) and read its `rituals.md`.

2. **Find the row** matching `$ARGUMENTS`. No match, or no argument → list the
   rituals (name, cadence, executor) and stop. From the row take the
   **executor** (an org role), the **procedure** (a skill ref or inline steps),
   and the **cadence** — the freshness window for any `metrics/actuals/` data
   involved (never reason from data older than it; spec rule 5).

3. **Delegate to act**: perform the `/trellis:act` procedure with the executor
   as the role and the ritual's procedure as the input — resolve the mandate,
   record the acting role in `.trellis/acting-role`, adopt the holder, execute
   within authority, remove the marker when done. When the executor's holder is
   a ref to the `trellis:steward` agent, invoke that agent, telling it which
   ritual to run; collect its escalations.

4. **Deliver escalations** through the instance's escalation channel (see
   `conventions.md` → "Runtime binding"; default: forge issues assigned to the
   owner or `escalate-to:` role's holder, one per finding, via
   `gh issue create`). Without forge access, list them fully in the report,
   addressed to their owners.

5. **Report**: ritual name, executor, when it last ran if determinable (git
   history of generated views), findings, escalations opened (refs), artifacts
   regenerated, and any SLA the escalations carry per `rituals.md`.
