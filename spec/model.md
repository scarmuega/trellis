# Trellis — specification (v13)

> *A domain-driven operating model for businesses run by humans and AI agents.*

Trellis is an operating model for a business run by humans and AI agents
together, structured as one versioned filesystem of markdown artifacts per
domain. Everything the business knows and does gets an artifact with an owner,
a provenance, and a lifecycle: the market needs it serves, the strategies
committed to fulfill them, the problem space those commitments induce, the
solution contexts operated in response, the plans, the metrics, the decisions,
and the organization itself. The repo is the organization: agents load
artifacts as their instructions, humans review diffs as their governance, and
both work in the same medium.

It exists because AI agents changed the economics of disciplined operation.
Agents can now execute business processes end-to-end — but delegating to them
demands explicit authority, stateless sessions demand predictable retrieval,
and automated execution demands knowledge kept current. Operating models of
this fidelity used to die of documentation ceremony; when the documentation is
the execution medium, the upkeep pays for itself. Trellis is the stable shape
that makes this workable: a place for every kind of business knowledge, known
in advance to every participant, human or synthetic.

The specification spans a set of documents: **Rationale** (`spec/rationale.md`)
— the premises, how the world is and why this framework follows from it;
**Hierarchy**, **Schemas**, and **Rules** (this document) — the shape and the
obligations of operating under it; **Patterns** (`spec/patterns.md`) — recurring
practice, advisory only; and the **runtime companion** (`spec/runtime.md`) — how
a domain is operated day to day.

## Hierarchy

```
/                                   => repo root = the domain
├── conventions.md                  => provenance classes, frontmatter schemas, secrets policy
├── rituals.md                      => heartbeat: reviews, metric sweeps, standing processes,
│                                      escalation SLAs — executed by mandated agents
├── glossary.md                     => ubiquitous language; agents must cite terms from here
├── market.md                       => founding map: the invariant layer — needs (anchored
│                                      ## N-{slug}), jobs-to-be-done, market, personas —
│                                      technology-free; endures pivots
├── problem/
│   └── {subdomain}.md              => induced problem space; frontmatter: induced-by edges naming
│                                      the strategy that put it here, class per edge — business
│                                      functions live here, classified per strategy
├── strategy/
│   └── {strategy}.md               => a committed solution to a need — the business model as
│                                      operated, not as aspired to; frontmatter: status (the
│                                      maturity ladder; only the committed band induces), need
│                                      ref, differentiation claim, funding edges (funded-by —
│                                      what sustains it), core-ranking
├── brand.md                        => promise to customers — the customer-facing voice of the
│                                      committed strategy's differentiation claim
├── economics.md                    => pricing, revenue model, unit economics — the narrative
│                                      over the funded-by skeleton in strategy/
├── metrics/
│   ├── definitions.md              => authored: metric defs, targets, owners (via mandates)
│   └── actuals/                    => generated: bot-committed snapshots or state-refs
├── plans/
│   └── {plan}.md                   => frontmatter: status + type; campaigns and initiatives
│                                      are plans like any other
├── decisions/
│   └── NNNN-{slug}.md              => ADRs, append-only
├── solution/                       => solution space — business and technical alike
│   ├── context-map.md              => internal BCs + external contexts, DDD relations annotated
│   │                                  (customer-supplier | conformist | acl | published-language)
│   └── {bounded context}/          => a language boundary: sales, support, and indexing
│       │                              qualify equally
│       ├── README.md
│       ├── glossary.md             => BC-local terms; overrides domain glossary
│       ├── contracts/              => published language: tx3, IDL, OpenAPI, brand guidelines
│       ├── skills/
│       │   └── {skill}/            => ALL procedures live here — business playbooks and
│       │                              technical runbooks through one mechanism
│       └── {deployment unit}/
└── org/
    └── {role}/                     => a mandate plus whoever or whatever holds it;
        │                              humans and agents through one schema
        ├── mandate.md              => ALWAYS local: purpose, scope, authority, escalation
        └── holder/                 => the identity filling the role:
            │                          EITHER a local agent package:
            ├── system.md           =>   system prompt implementing the mandate
            ├── tools/
            ├── skills/
            ├── evals/              =>   mandate compliance as test cases
            │                          OR a single opaque ref to an external identity:
            └── ref.md              =>   for imported agents and for humans
```

## Frontmatter schemas (defined in conventions.md)

### Every artifact
```yaml
provenance: authored | generated | state-ref
owner: org role ref                 # who may edit; agents check before writing
tags: [mktg, growth]                # groupings are tags; views over tags are generated
```

### problem/{subdomain}.md
```yaml
provenance: authored
induced-by:                              # >=1 edge; no edge to a committed
  - strategy: strategy/{strategy}.md     #   strategy => orphan (lint)
    class: core | supporting | generic   # classification is a property of the edge
# effective automation policy = strictest class across edges to COMMITTED
# strategies (core > supporting > generic; orphans default to core) — this
# mapping is canonical here; it applies to business functions too:
#   generic    => full agent autonomy (or buy it)
#   supporting => agent executes, human samples
#   core       => human review gate on every change
```

### strategy/{strategy}.md
```yaml
provenance: authored
status: raw | defined | validated | implemented | established | discarded
    # the maturity ladder — each stage names the work owed: refine, validate,
    # implement, monitor, harden; discarded is terminal (failed, lacked merit,
    # or superseded — the successor's supersedes: marks the last). Bands carry
    # the coarse vocabulary: committed = validated|implemented|established —
    # only the committed band induces (rule 11); aspirational = raw|defined;
    # retired = discarded.
need: market.md#n-{slug}                     # the market need this fulfills
differentiation: one line — why we win, against which alternative
funded-by:                                   # what sustains this strategy (rule 12);
  - strategy: self                           #   captures its own revenue, OR
  - strategy: strategy/{funder}.md           #   another strategy captures for it, OR
    relation: current | intended             #   an external ref (rule 2). relation
                                             #   defaults to current.
    # the edge declares the operating model's designed value flow — who captures
    # the value this strategy produces. How real the operation is stays on the
    # maturity ladder; how capture performs stays in metrics; economics.md
    # narrates the skeleton these edges draw. current = sustains under the model
    # as committed; intended = a conversion thesis — legal toward any stage,
    # never sustaining. Committed band with no current edge to self, a
    # committed strategy, or an external ref = economic orphan (lint).
core-ranking: [problem/{a}.md, problem/{b}.md]  # required iff >1 core edge under
                                                # this strategy; a total order,
                                                # scarcest attention first
supersedes: strategy/{previous}.md           # optional; set on pivot
```

### plans/{plan}.md
```yaml
provenance: authored
status: draft | active | blocked | retired
type: initiative | campaign | experiment | ...   # open set, defined in conventions.md
subdomains: [problem/growth.md]
contexts: [solution/marketing]      # the bounded context(s) this plan executes through
metrics: [metrics/definitions.md#metric]
decisions: [decisions/0004-*]
```

### mandate.md (humans and agents — never imported)
```yaml
purpose: one line
scope: [subdomain refs]
authority:
  spend: {limit, period}
  publish: [channels]
  approve: [artifact globs]
escalate-to: org role ref
holder: holder/ | <opaque external ref>   # identity is portable; authority is not
```

## Rules

1. **One repo = one root = one domain, and the root is the boundary.** Everything
   under it is the domain's own knowledge, state-refs, and org. The framework's
   contract ends at the root: how artifacts are shared, imported, published, or
   aggregated across roots is out of scope — consumers solve it with their tooling
   of choice.
2. **Refs are the only boundary-crossing primitive.** A ref is an opaque name/URI
   inside an artifact, pointing at external contexts, identities, skills, or systems
   of record. Refs are knowledge ("this relates to that"); resolution is the
   consumer's tooling. The framework guarantees only that refs are declared where
   they're used.
3. **External relationships are knowledge, not plumbing.** `context-map.md` records
   which external contexts this domain depends on and the DDD relation to each —
   regardless of how (or whether) their artifacts are physically present.
4. **Mandates are always local.** Identities may live anywhere; scope and authority
   are meaningless outside the domain and never cross the root. One identity holding
   roles in N domains means N local mandates.
5. **State lives elsewhere.** `metrics/actuals/` is generated on the ritual cadence or
   held as state-refs; agents never reason from numbers older than the interval.
6. **Decisions are append-only.** Supersede, never edit — shared memory across agent
   generations.
7. **Directories hold kinds; groupings are tags.** A branch earns a directory only by
   introducing a new artifact kind — something needing its own frontmatter: its own
   lifecycle, owner, and provenance distinct from any parent document. Everything else
   is body content; refs can target anchors, so linkability alone never justifies
   promotion. Cross-cutting collections are generated views over tags, never a second
   authored hierarchy.
8. **A unit that needs its own goals, metrics, and plans is a domain.** Give it a
   root. Anything less creates a shadow domain inside this one.
9. **The framework is fractal.** A portfolio, holdco, or platform is just another
   domain with this same shape; whatever composes them is, by rule 1, out of scope.
10. **Names are retrieval keys.** Every name — in the framework and in an instance's
    subdomains, contexts, skills, and roles — lands in an agent's context window.
    Prefer distinctive tokens over ambient ones; expand, don't abbreviate: no opaque
    acronyms.
11. **The domain map carries its derivation.** Every `problem/{subdomain}.md`
    declares the strategy edges that induced it; classification lives on the edge;
    only `committed` strategies induce (the band
    `validated | implemented | established` — see the strategy schema). An
    untagged subdomain is a modeling
    error — it either belongs in the technology-free founding map (`market.md`)
    or it owes its existence to a strategy and must say which. Retiring a strategy
    orphans the
    subdomains it induced — each must be re-parented to a surviving commitment or
    archived.
12. **A strategy declares what sustains it.** Strategies decompose by need and
    differentiation, not by revenue stream, so producing value and capturing it
    routinely land in different strategies. Every `committed` strategy carries
    `funded-by:` edges naming where the value it produces is captured: itself
    (`self`), another strategy, or an external ref (rule 2). Semantics ride the
    edge: `relation: current` is the operating model as committed;
    `relation: intended` is a conversion thesis — it documents, never sustains,
    and edges to aspirational strategies do the same. A committed strategy with
    no sustaining edge — no `current` edge to `self`, a committed-band
    strategy, or an external ref — is an economic orphan: a modeling error
    mirroring rule 11's untagged subdomain. Discarding a strategy economically
    orphans its dependents exactly as it orphans induced subdomains — each must
    be re-funded, converted, or reconsidered. Economic lineage is
    strategy-layer knowledge: `economics.md` narrates the skeleton the edges
    draw; the founding map never carries it.

For the premises these rules answer to, see `spec/rationale.md`. For recurring,
non-normative practice inside this structure, see `spec/patterns.md`.
