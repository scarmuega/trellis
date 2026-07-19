---
provenance: authored
owner: org/founder
status: blocked
type: initiative
subdomains: [problem/categorization-accuracy.md]
contexts: [solution/ledger-engine]
metrics: [metrics/definitions.md#categorized-transactions-rate]
---
# Integrate bank feeds

Status note: blocked since %%DAYS_AGO_5%% — waiting on a sandbox API key from
the bank-feed vendor; requesting one is a fifteen-minute support ticket.

## Objective

Pull transactions from live bank feeds so categorization-model training runs
on real data — the lever behind `categorized-transactions-rate`, the
top-ranked core problem in `strategy/automated-bookkeeping.md`.

## Approach

Connect the bank-feed vendor's sandbox, replay a month of live transactions
through the ledger engine, and retrain categorization on the result.

## Measurement

`categorized-transactions-rate` against the 90% target.
Horizon: ends %%DAYS_AHEAD_60%%.

## Risks and escalation triggers

If the vendor sandbox drops more than 1% of replayed transactions, escalate to
org/founder before retraining.
