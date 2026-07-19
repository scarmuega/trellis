---
provenance: authored
owner: org/founder
status: blocked
type: initiative
subdomains: [problem/booking-pipeline.md]
contexts: [solution/booking-desk]
metrics: [metrics/definitions.md#booking-throughput]
---
# Migrate booking engine

Status note: blocked since %%DAYS_AGO_45%% — the incumbent vendor's contract
renegotiation stalled; legal review pending on both sides.

## Objective

Move the booking pipeline off the incumbent vendor's engine onto the booking
desk, removing the per-booking ceiling that caps weekly throughput.

## Approach

Renegotiate exit terms with the incumbent, then migrate live bookings in
cohorts to the in-house engine. Migration cannot start until the contract
terms clear legal review on both sides.

## Measurement

`booking-throughput` against the 80-per-week target.
Horizon: ends %%DAYS_AHEAD_60%%.

## Risks and escalation triggers

If any migrated cohort loses a confirmed booking, escalate to org/founder and
halt the cohort migrations.
