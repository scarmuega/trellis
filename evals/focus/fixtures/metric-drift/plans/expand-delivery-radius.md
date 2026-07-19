---
provenance: authored
owner: org/founder
status: active
type: initiative
subdomains: [problem/order-volume.md]
contexts: [solution/dispatch]
metrics: [metrics/definitions.md#weekly-orders]
---
# Expand delivery radius

## Objective

Lift weekly order volume toward the 300-per-week target by opening
two adjacent neighborhoods to the same-day slot promise.

## Approach

Onboard one partner florist per new neighborhood and extend courier routes
outward from the current core, keeping slot density high enough to hold
the promise while the radius grows.

## Measurement

`weekly-orders` toward 300 per week, read from `metrics/actuals/latest.md`.
Horizon: ends %%DAYS_AHEAD_21%%.

## Risks and escalation triggers

Escalate to org/founder if `weekly-orders` falls below 120 in any snapshot.
