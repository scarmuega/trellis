# Trellis — specification (v11)

> **The structure your business grows along.**
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

The specification reads in order: **Rationale** — the premises, how the world
is and why this framework follows from it; **Hierarchy** and **Schemas** — the
shape; **Rules** — the obligations of operating under it; **Patterns** —
recurring practice, advisory only.

## Rationale

1. **Needs are primitive; a business is a search for a viable strategy to fulfill
   them.** Needs exist in the market independently of any business — external,
   slow-changing, technology-free: the model's only invariant layer. A strategy is
   a committed solution to a need — the business model as actually operated, not
   as aspired to. Only commitments induce consequences; intentions do not. The
   search is not optimization of a fixed objective: strategy is chosen and
   revocable, and the search must sustain itself economically to continue —
   viability is knowledge in its own right.
   *Grounds: `problem/README.md` (the founding map), `strategy/` and its status
   lifecycle, `economics.md`, `brand.md`.*

2. **The domain is induced, not found.** `need × strategy → domain`: the domain is
   everything the business must become expert at *because of how* it chose to
   fulfill the need. Aircraft maintenance is in an airline's domain because the
   strategy put it there — an induced subdomain owes its existence to a
   commitment, and to nothing else. Induction also sets the asymmetry of change:
   needs edits are rare and cascade everywhere; strategy edits are the normal
   unit of pivot.
   *Grounds: the founding-map / induced-subdomain split inside `problem/`,
   `induced-by:` edges, `supersedes:` on `strategy/`.*

3. **Problem and solution are roles, not territories.** The model is stratified: a
   solution at level N becomes the problem context at level N+1. Strategy is a
   solution at the level of the market and a problem-generator at the level of
   operations. `problem/` and `solution/` name the roles at the top stratum, not
   two fixed regions.
   *Grounds: `strategy/` between `problem/` and `solution/`, the stratum-role
   names themselves, fractal roots.*

4. **Subdomain classification is relative to strategy, and induced subdomains are
   real.** Core is where the strategy claims differentiation; supporting is what
   the strategy drags in but doesn't compete on; generic is where the induced
   problems coincide with everyone else's, so the market already solved them. The
   same problem classifies oppositely under different strategies (payments:
   generic for almost everyone, core for Stripe). And once induced, subdomains are
   genuine problem spaces — their own experts, language, failure modes, and
   economics — never mere implementation detail: banning technology-flavored
   subdomains from the map hides real operational problems that consume real
   expertise.
   *Grounds: class-on-edge, the strictest-wins effective policy, `core-ranking:`,
   admission of technology-flavored subdomains.*

5. **The search runs on imperfect information.** The problem space is a set of
   beliefs to be continually re-evaluated; the solution space is a set of
   experiments to be improved. The framework is therefore an epistemic machine, not
   a filing system: knowledge must be revisable with an audit trail, beliefs at
   decision time must be recorded, feedback must flow on a cadence, and execution
   must carry status.
   *Grounds: versioned artifacts, `decisions/`, `metrics/`, `rituals.md`,
   plan lifecycle.*

6. **AI agents collapse two costs at once**: the cost of executing business
   processes previously limited to humans, and the cost of maintaining the
   structured knowledge that disciplined operation requires. The second matters
   most — methodologies of this fidelity have always died of documentation
   ceremony. When the documentation is the execution medium (the artifact an agent
   loads *is* the process), upkeep stops being overhead.
   *Grounds: automation policy derived from strategy edges, `skills/`, holder
   packages, agent-executed rituals.*

7. **Under agents, the entire solution space becomes software-shaped.** Every
   function — marketing, sales, support, finance — is operationalized through
   agents, tools, and contracts. This is what licenses borrowing DDD wholesale:
   DDD is software engineering's mature discipline for partitioning *meaning* at
   scale, and once every function is software, its applicability is total, not
   partial.
   *Grounds: glossaries, bounded contexts, `context-map.md`, per-strategy
   core/supporting/generic classification, the absence of a functions branch.*

8. **Versioned natural language is the shared medium.** Code is native to machines;
   conversation is native to humans; versioned natural-language text is the
   intersection both can read, write, diff, and review. A filesystem of markdown is
   therefore the organizational substrate.
   *Grounds: the substrate choice itself.*

9. **Structure substitutes for memory.** Agents are stateless — each session
   reconstructs its world from what it can retrieve. Humans onboard, forget, and
   leave. A well-known, stable shape makes retrieval predictable for both: every
   participant knows where any kind of knowledge lives without being told. The
   framework is the organization's type system.
   *Grounds: the fixed hierarchy, the naming principle, kinds-vs-tags,
   one schema for humans and agents.*

10. **Delegated execution requires explicit authority.** Automating what humans did
    means deciding what agents may decide. Bounded, local, auditable mandates make
    delegation safe rather than implicit.
    *Grounds: `mandate.md`, the mandate/holder split, always-local authority.*

Corollary: every element of the framework must be grounded by some premise above;
an element grounded by none is a candidate for deletion. Premises state how the
world is; rules state how to operate under it — an obligation in a premise, or
worldview in a rule, is misfiled. Grounds name elements, never rules or their
enforcement: rules answer to premises, not the reverse.

## Hierarchy

```
/                                   => repo root = the domain
├── conventions.md                  => provenance classes, frontmatter schemas, secrets policy
├── rituals.md                      => heartbeat: reviews, metric sweeps, standing processes,
│                                      escalation SLAs — executed by mandated agents
├── glossary.md                     => ubiquitous language; agents must cite terms from here
├── problem/
│   ├── README.md                   => founding map: needs (anchored ## N-{slug}), jobs-to-be-done,
│   │                                  market, personas — technology-free; endures pivots
│   └── {subdomain}.md              => induced problem space; frontmatter: induced-by edges naming
│                                      the strategy that put it here, class per edge — business
│                                      functions live here, classified per strategy
├── strategy/
│   └── {strategy}.md               => a committed solution to a need — the business model as
│                                      operated, not as aspired to; frontmatter: status (only
│                                      committed induces), need ref, differentiation claim,
│                                      core-ranking
├── brand.md                        => promise to customers — the customer-facing voice of the
│                                      committed strategy's differentiation claim
├── economics.md                    => pricing, revenue model, unit economics
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
status: aspirational | committed | retired   # only committed induces
need: problem/README.md#n-{slug}             # the market need this fulfills
differentiation: one line — why we win, against which alternative
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
    only `committed` strategies induce. An untagged subdomain is a modeling
    error — it either belongs in the technology-free founding map or it owes its
    existence to a strategy and must say which. Retiring a strategy orphans the
    subdomains it induced — each must be re-parented to a surviving commitment or
    archived.

## Patterns & practices (non-normative)

Recurring content that lives *inside* the structure above. These are descriptions of
practice, not rules: adopt, adapt, or ignore per domain. If one of these ever acquires
independent lifecycle in your instance, rule 7 tells you when it earns promotion.

### Business functions
Marketing, sales, support, finance are not a branch — they dissolve into the
ontology. The function's problem is a subdomain, classified core/supporting/generic
per strategy edge, which is what puts business functions under the automation
policy. Its solution is a bounded context. Its campaigns and initiatives are plans.
Its standing processes are rituals. Its ownership is a mandate. "Everything mktg"
is a generated view over tags, per rule 7.

### Requirements
Requirements are a genre, not a kind — they dissolve into the ontology by nature.
Constraints and needs are problem-space: body of `problem/{subdomain}.md`. Functional
specifications are solution-space: body of the bounded context's README. Acceptance
criteria belong to the plan that delivers them. A requirement never has a lifecycle of
its own; it inherits its parent's.

Practices: give requirement statements stable anchored headings in the body
(`## R-onboarding-3`) so plans, contracts, and evals can ref them without a
requirements directory. Treat prose as the larval form — the durable forms are
executable: a contract (tx3, IDL) is a requirement made machine-checkable; an eval is
a requirement on an agent made testable. The methodology's pressure is to hatch prose
into `contracts/` and `evals/`, not to accumulate it.

### Personas & market
Personas are founding-map knowledge — demand-side and technology-free, so they
endure pivots: sections within `problem/README.md` or the relevant subdomain file —
who they are, needs, context, willingness to pay — each under a stable anchor so
brand, plans, and metric definitions can ref them. A persona's needs connect to the
map's `## N-{slug}` need anchors. Persona names enter `glossary.md` as ubiquitous
language: when an agent writes "the indie dev," it must resolve to one canonical
definition.

Market splits across the existing spine rather than getting a home: segmentation and
competitive landscape are problem knowledge (body of `problem/README.md`); sizing and
pricing-per-segment are `economics.md` content. "Market" as a directory would be a
grouping, not a kind.

### User journeys
An as-is journey — how a persona currently experiences the problem, where it hurts —
is discovery knowledge that belongs to the persona: a section next to their
definition, anchor-referenceable. A to-be journey is a goal, and goals already have a
kind: it's a plan (`type: journey`), where the `contexts:` refs do real analytical
work — a journey's value is precisely that it crosses bounded-context seams. The
journey is the demand-side reading of `context-map.md`.

### Vendors
Split by what the vendor provides. A vendor providing *a system* (a card processor,
a cloud, an API provider) is an external bounded context: an entry in
`context-map.md` with its DDD relation — conformist for SaaS you adapt to,
customer-supplier for contracted integrations, ACL for vendors you wrap to protect
your model. A vendor providing *judgment or labor under delegated authority* (the
accountant, an agency, outsourced support) is a role with an external holder:
`org/{role}/mandate.md` pins scope, spend authority, and escalation exactly as it
would for an agent; `holder/ref.md` points at the party. The residue — signed
contracts, pricing, renewal dates — is body content plus state-refs to the system of
record, with renewals as `rituals.md` entries.

### Pivot
A pivot is a strategy edit, not a rewrite of the world. The founding map endures —
if it doesn't, that wasn't a pivot but a different business. Procedure: set the old
strategy's `status: retired`; commit the new one, `supersedes:` naming the old.
Every subdomain the retired strategy induced is now orphaned (rule 11): the
derivation sweep flags each for its owner to re-parent onto a surviving commitment
(often with a different `class:` — classification is per edge) or archive. Orphans
carry core policy until resolved, so a half-finished pivot tightens agent autonomy
rather than loosening it, and a root left with only retired strategies is flagged
as an incomplete pivot. Abandon agents for elves; the agent-orchestration
subdomain collects itself.

### Stratified alternation
The needs → strategy → subdomains machinery is normative only at the top stratum,
but the logic recurs: each level's committed solution is the next level's problem
context. Below the top stratum, a bounded context's significant commitments (a
runtime, a vendor, an architecture) are `decisions/` whose Consequences name what
they induce; superseding the decision orphans those consequences — same garbage
collection, lighter machinery. When a lower stratum genuinely needs its own goals,
metrics, and plans, rule 8 fires: it becomes a new domain root, where the full
machinery applies again. Recursion happens via fractal roots (rule 9), never
deeper trees.

### Classification inversion (portfolio)
Across ventures sharing a substrate, check each generic subdomain consumed by
venture A for being a core subdomain of some venture B — in the portfolio or the
market. The inversion is healthy and expected: it marks a clean factoring boundary
and a candidate internal interface (mandate-governed agent payments: one venture's
core, a sibling's generic — same problem, opposite classification, two doors
down). Its absence is the smell: ventures duplicating each other's core, or one
accidentally building a sibling's product as "internal tooling." This check runs
at a portfolio root (rule 9: a portfolio is just a domain) over its venture refs;
how classifications reach that root is the consumer's tooling, per rule 1.
