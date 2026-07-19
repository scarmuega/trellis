---
provenance: authored
owner: org/founder
status: active
type: initiative
subdomains: [problem/delivery-operations.md]
contexts: [solution/dispatch]
metrics: [metrics/definitions.md#on-time-rate]
---
# Tighten delivery slots

## Objective

Raise the share of deliveries landing inside the promised slot to the 95%
target by shortening the promised window without breaking it.

## Approach

Re-sequence courier routes around slot deadlines rather than distance, and
hold back slot commitments once a route's remaining capacity would put its
existing promises at risk.

## Measurement

`on-time-rate` toward 95% inside the promised slot, read from
`metrics/actuals/latest.md`. Horizon: ends %%DAYS_AHEAD_45%%.

## Risks and escalation triggers

Escalate to org/founder if `on-time-rate` drops below 70% in a fresh snapshot.
