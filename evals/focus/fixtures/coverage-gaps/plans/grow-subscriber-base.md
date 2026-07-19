---
provenance: authored
owner: org/founder
status: active
type: campaign
subdomains: [problem/subscriber-growth.md]
contexts: [solution/box-operations]
metrics: [metrics/definitions.md#subscriber-count, metrics/definitions.md#churn-rate]
---
# Grow subscriber base

## Objective

Reach the 500-subscriber target while pushing monthly churn under 5%,
so the box cadence is funded by recurring revenue alone.

## Approach

A referral campaign through existing subscribers: one free box per converted
referral, paired with a win-back offer sent at the first skipped renewal.

## Measurement

`subscriber-count` toward 500 and `churn-rate` toward below 5% monthly,
read from `metrics/actuals/latest.md`. Horizon: ends %%DAYS_AHEAD_30%%.

## Risks and escalation triggers

Escalate to org/founder if `churn-rate` rises above 8% in any snapshot.
