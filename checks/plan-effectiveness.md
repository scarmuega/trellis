# Plan-effectiveness checklist

Canonical plan-effectiveness checks for a Trellis domain root: does the
solution, as planned, move the problem, per the metrics? Referenced by the
`trellis:focus` agent (the focus ritual) and the `/trellis:focus` command —
this file is the single source; keep both pointing here rather than copying the
list. Two deliberate departures from the conventions lint: these are judgment
checks, not pass/fail — each item names a condition worth an owner's attention,
and the evidence decides, never the pattern-match; and every finding is
advisory — it proposes, the owner decides.

**Evidence discipline.** `metrics/actuals/` older than the freshness window in
`rituals.md` is never evidence (spec rule 5). Staleness is its own finding
(item 5), and no judgment that would have rested on the stale number gets made.

**The finding record.** Every finding — in a session report or an escalation
body — carries this block, fenced as `yaml`. It is what makes findings
addressable to an owner, dedupable across runs, and gradeable by the plugin's
evals (`evals/focus/`):

```yaml
kind: candidate | gap | blocker | risk | challenge
item: 7                        # the checklist item that fired
refs: [plans/{plan}.md, problem/{subdomain}.md]
evidence: one line citing the numbers and files that ground it
action: one line — the suggested move
owner: org/{role}              # addressed-to
```

The record travels verbatim: whatever relays a finding — a ritual runner, an
escalation body, a session report — carries the fenced block unmodified
alongside its prose. A relay that summarizes findings in its own words has
dropped the contract.

Walk the items in order — the coverage walk sets the attention ranking the rest
report under. "Active plans" below means status `active` or `blocked`; nothing
evaluates a draft.

## Coverage — walk `market.md` needs → committed strategies → induced subdomains → active plans

1. Every committed strategy has at least one active plan advancing it. A
   strategy no plan advances is operating on inertia — `gap`.
2. Every core subdomain has an active plan, checked in `core-ranking` order. An
   unplanned top-ranked core subdomain outranks every other finding of this
   walk — `gap`.
3. Every defined metric is ref'd by some active plan, and every active plan
   refs at least one metric. A metric nobody moves is a `gap` — or a dead
   definition, a `challenge` to its owner. A metricless active plan cannot be
   evaluated at all — `challenge`.

## Metric effectiveness — per active plan

4. The plan's `metrics:` refs against fresh actuals and targets: flat or
   opposing movement inside the plan's own measurement horizon — `challenge`.
5. Actuals missing or stale for a plan's metrics: the domain is flying that
   plan blind — `blocker`. A root that has never recorded actuals gets one
   aggregate finding, not one per plan.

## Attention allocation — `core-ranking` is a spend order for scarce attention

6. Sustained active-plan effort on low-ranked or supporting subdomains while a
   higher core subdomain sits unplanned — `candidate`: redirect.
7. A generic subdomain consuming authored plan effort contradicts its own
   policy — full agent autonomy, or buy it — `candidate`: automate, delegate,
   or buy.
8. A blocked plan whose blocker is cheap to clear relative to its stake — a
   high-ROI `candidate`: name the clearing action.

## Blockers and risks

9. `status: blocked` older than one ritual cadence with no recorded movement
   (git history) — clear it, replan, or retire: `blocker`.
10. A plan body's risks-and-escalation-triggers section whose trigger condition
    is now met by actuals — the escalation the plan itself promised; cite the
    trigger verbatim — `risk`.
11. Two active plans pulling the same metric or subdomain in opposing
    directions — `risk`, addressed to both owners.

## Challenge — every active plan answers for its worth

12. Each active plan answers: which need or strategy does this serve, and what
    does the metric say? A plan that answers with structure but not value —
    refs resolve, numbers don't move, nobody would miss it — is value-dead:
    retire or rejustify — `challenge`.
13. A plan past its measurement horizon without a recorded decision, and any
    experiment whose decision criterion has resolved, owes its owner a
    verdict — continue, graduate, or retire: `challenge`.

## Maturity — the stage names the work owed and the metrics that read it

Every strategy past `raw` enters this group, whatever its band (Strategy
maturity pattern); `raw` is exempt — its work is refining the artifact, which
no plan carries. On a root still pinned to a three-value status enum (spec
v11), skip this group.

14. Stage-work is planned: `defined` has an active `experiment` with a
    decision criterion serving it (item 12's attribution); `validated`,
    active implementation work; `established`, active hardening or
    optimization work. Missing — `gap`. For committed-band strategies this
    sharpens item 1 rather than duplicating it: name the stage-matched plan
    type in the action line. An active plan advancing a `discarded` strategy
    is the inverse — `challenge` to the plan's owner: retire or rejustify.
15. The metrics reading a strategy's plans match its stage: learning and
    leading indicators for `defined`, delivery progress for `validated`,
    outcomes against `definitions.md` targets for `implemented`, efficiency
    and unit economics for `established`. A mismatch — outcome targets graded
    against a strategy still validating, or an `implemented` strategy with no
    outcome metric defined — `challenge`.

## Economic lineage — `funded-by:` edges are the capture skeleton `economics.md` narrates

A strategy's plans answer not only to its own metrics but to whoever captures
the value they produce (spec rule 12). On a root whose pinned spec predates
`funded-by:` (v12 or earlier), skip this group.

16. Attention on a value-producing strategy is challengeable against the
    health of its capture: for each committed strategy sustained by *another*
    strategy (`current` edges, not `self`), read the funder's stage, plans,
    and metrics. Active plans pouring effort into the producer while its
    funder is stalled, unplanned (item 1), or missing its targets is value
    nobody collects — `risk`, addressed to both owners.
17. An `intended` edge is a conversion thesis on the current funder's clock:
    the target should be climbing the maturity ladder while the subsidy
    lasts. A conversion target still aspirational with no active plan
    advancing it, while the dependent's only sustaining edge is a single
    `current` funder — the thesis is aging as the subsidy runs — `gap`. The
    funder's own stage or metrics deteriorating sharpens it: the funding
    strategy may die before the conversion lands — `risk`, addressed to the
    owners of both the dependent and the target.
