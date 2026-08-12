---
description: Execute a ritual from rituals.md as its mandated executor role
argument-hint: <ritual name>
---

Execute one ritual from the domain's `rituals.md` — the scheduled plane of the
runtime contract (the trellis plugin's `spec/runtime.md`;
`${CLAUDE_PLUGIN_ROOT}/spec/runtime.md` where the plugin is installed). A
ritual is `act(executor, procedure)`: this procedure looks up the row and
delegates to the `/trellis:act` procedure (rendered below this one under
dispatch).

Input — the **ritual name**, matched case-insensitively against the ritual
column of `rituals.md`. Invoked as `/trellis:ritual`, `$ARGUMENTS` supplies
it. Invoked under dispatch, the preamble above this procedure names it and the
resolved executor.

## Procedure

1. **Bind to the domain root** (nearest directory at or above the working
   directory holding `conventions.md`, `problem/`, `solution/`, and `org/`; not
   found → say so and stop) and read its `rituals.md`.

2. **Find the row** matching the ritual name. No match, or no name → list the
   rituals (name, cadence, executor) and stop. From the row take the
   **executor** (an org role), the **procedure** (a skill ref or inline steps),
   and the **cadence** — the freshness window for any `metrics/actuals/` data
   involved (never reason from data older than it; spec rule 5). Under
   dispatch the preamble already names the executor; still read the row for
   the procedure and the cadence.

3. **Delegate to act**: perform the `/trellis:act` procedure with the executor
   as the role and the ritual's procedure as the input — resolve the mandate,
   record the acting role in `.trellis/acting-role`, adopt the holder, execute
   within authority, remove the marker when done. When the executor's holder is
   a ref to a plugin agent (`trellis:steward`, `trellis:focus`), invoke that
   agent, telling it which ritual to run; collect its escalations.

4. **Deliver escalations** through the instance's escalation channel (see
   `conventions.md` → "Runtime binding"; default: escalation records in the
   artifact each finding concerns). The steward and the focus own no authored
   artifact, so a ritual they execute delivers its findings as report content,
   one per finding, addressed to the owner who transcribes it into that
   artifact's `## Escalations` section. An executor that *does* own the artifact
   writes the record itself. Any structured finding record the executor produced
   (e.g. the plan-effectiveness fenced `yaml` block) travels verbatim into the
   escalation record or the report — never resummarized.

5. **Report**: ritual name, executor, when it last ran if determinable (git
   history of generated views), findings, escalations raised — where each record
   landed, or which owner owes its transcription — artifacts regenerated, and any
   SLA the escalations carry per `rituals.md`.
