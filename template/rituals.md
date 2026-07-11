---
provenance: authored
owner: <owner>
---
# Rituals

The domain's heartbeat. Each ritual declares cadence, executor (an org role), the
procedure (a skill or inline steps), and where deviations escalate.

| ritual           | cadence | executor    | procedure                                          |
|------------------|---------|-------------|----------------------------------------------------|
| conventions lint | weekly  | org/steward | run lint checklist; open escalations on violations |
| metric sweep     | weekly  | org/steward | diff metrics/actuals vs targets; annotate plans    |
| plan review      | weekly  | <owner>     | walk plans with status active or blocked           |
