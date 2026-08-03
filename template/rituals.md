---
provenance: authored
owner: <owner>
---
# Rituals

The domain's heartbeat. Each ritual declares cadence, executor (an org role), the
procedure (a skill or inline steps), and where deviations escalate — as escalation
records in the artifact each finding concerns, written by that artifact's owner
(`conventions.md` → "Escalation records"). The `metrics/actuals/` freshness window
equals the metric-sweep cadence unless stated otherwise here.

| ritual           | cadence | executor    | procedure                                          |
|------------------|---------|-------------|----------------------------------------------------|
| conventions lint | weekly  | org/steward | run lint checklist; report violations to each artifact's owner to record |
| metric sweep     | weekly  | org/steward | diff metrics/actuals vs targets; report deviations to plan owners |
| derivation sweep | weekly  | org/steward | on founding-map/strategy/contract edits: escalate downstream artifacts and consumer contexts for revalidation; flag orphaned subdomains and economically orphaned strategies |
| focus            | weekly  | org/focus   | evaluate ready, active, and blocked plans against problem and metrics (plan-effectiveness checklist); report findings to owners to record |
| plan dispatch    | daily   | org/steward | scan `status: ready` plans, skipping any whose `awaits:` targets are not all retired (held, not blocked); `act(owner)` each to advance it (owners' acts flip ready→active); deterministic scan, work runs under each owner's authority |
