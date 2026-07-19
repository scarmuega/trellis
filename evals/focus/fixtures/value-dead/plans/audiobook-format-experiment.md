---
provenance: authored
owner: org/founder
status: active
type: experiment
subdomains: [problem/reader-audience.md]
contexts: [solution/storefront]
metrics: [metrics/definitions.md#audiobook-conversion]
---
# Audiobook format experiment

## Objective

Test whether an audiobook edition widens the reachable audience per storefront,
serving `market.md#n-reach-readers` through `strategy/direct-author-storefronts.md`.

## Approach

Produce audiobook editions for five pilot storefronts and surface them beside
the print and ebook listings, measuring what share of visitors buys the format.

## Measurement

`audiobook-conversion` against its 2% target.
Decision criterion: if audiobook-conversion has not reached 2% by %%DAYS_AGO_14%%, wind the experiment down.
Horizon: ends %%DAYS_AHEAD_15%%.

## Risks and escalation triggers

If audiobook-conversion falls below 0.5%, halt production spend and escalate to
org/founder. (Not met: the lowest snapshot on record is 1.1.)
