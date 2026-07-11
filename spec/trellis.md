# Trellis — specification (v10)

> **The structure your business grows along.**
> *A domain-driven operating model for businesses run by humans and AI agents.*

## Rationale

1. **A business is a search for problem-solution fit under economic constraints.**
   Not optimization of a fixed objective: the problem itself is being discovered,
   and the search must sustain itself economically to continue. Problem and solution
   are therefore the two first-class spaces of the framework, and viability is
   knowledge in its own right.
   *Grounds: `problem/`, `solution/`, `economics.md`, `brand.md`.*

2. **The search runs on imperfect information.** The problem space is a set of
   beliefs to be continually re-evaluated; the solution space is a set of
   experiments to be improved. The framework is therefore an epistemic machine, not
   a filing system: knowledge must be revisable with an audit trail, beliefs at
   decision time must be recorded, feedback must flow on a cadence, and execution
   must carry status.
   *Grounds: versioned artifacts, `decisions/`, `metrics/`, `rituals.md`,
   plan lifecycle.*

3. **AI agents collapse two costs at once**: the cost of executing business
   processes previously limited to humans, and the cost of maintaining the
   structured knowledge that disciplined operation requires. The second matters
   most — methodologies of this fidelity have always died of documentation
   ceremony. When the documentation is the execution medium (the artifact an agent
   loads *is* the process), upkeep stops being overhead.
   *Grounds: automation policy via subdomain class, `skills/`, holder packages,
   agent-executed rituals.*

4. **Under agents, the entire solution space becomes software-shaped.** Every
   function — marketing, sales, support, finance — is operationalized through
   agents, tools, and contracts. This is what licenses borrowing DDD wholesale:
   DDD is software engineering's mature discipline for partitioning *meaning* at
   scale, and once every function is software, its applicability is total, not
   partial.
   *Grounds: glossaries, bounded contexts, `context-map.md`, core/supporting/generic
   classification, the absence of a functions branch.*

5. **Versioned natural language is the shared medium.** Code is native to machines;
   conversation is native to humans; versioned natural-language text is the
   intersection both can read, write, diff, and review. A filesystem of markdown is
   therefore the organizational substrate.
   *Grounds: the substrate choice itself.*

6. **Structure substitutes for memory.** Agents are stateless — each session
   reconstructs its world from what it can retrieve. Humans onboard, forget, and
   leave. A well-known, stable shape makes retrieval predictable for both: every
   participant knows where any kind of knowledge lives without being told. The
   framework is the organization's type system.
   *Grounds: the fixed hierarchy, the naming principle, kinds-vs-tags,
   one schema for humans and agents.*

7. **Delegated execution requires explicit authority.** Automating what humans did
   means deciding what agents may decide. Bounded, local, auditable mandates make
   delegation safe rather than implicit.
   *Grounds: `mandate.md`, the mandate/holder split, always-local authority.*

Corollary: every element of the framework must be grounded by some premise above;
an element grounded by none is a candidate for deletion.

## Hierarchy

```
/                                   => repo root = the domain
├── conventions.md                  => provenance classes, frontmatter schemas, secrets policy
├── rituals.md                      => heartbeat: reviews, metric sweeps, standing processes,
│                                      escalation SLAs — executed by mandated agents
├── glossary.md                     => ubiquitous language; agents must cite terms from here
├── problem/
│   ├── README.md                   => domain problem statement
│   └── {subdomain}.md              => frontmatter: class: core|supporting|generic
│                                      business functions live here, classified per-business
├── brand.md                        => promise to customers
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
class: core | supporting | generic
# automation policy derives from this — and applies to business functions too:
#   generic    => full agent autonomy (or buy it)
#   supporting => agent executes, human samples
#   core       => human review gate on every change
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

## Patterns & practices (non-normative)

Recurring content that lives *inside* the structure above. These are descriptions of
practice, not rules: adopt, adapt, or ignore per domain. If one of these ever acquires
independent lifecycle in your instance, rule 7 tells you when it earns promotion.

### Business functions
Marketing, sales, support, finance are not a branch — they dissolve into the
ontology. The function's problem is a subdomain, classified core/supporting/generic
per business, which is what puts business functions under the automation policy. Its
solution is a bounded context. Its campaigns and initiatives are plans. Its standing
processes are rituals. Its ownership is a mandate. "Everything mktg" is a generated
view over tags, per rule 7.

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
Personas are problem-space knowledge: sections within `problem/README.md` or the
relevant subdomain file — who they are, needs, context, willingness to pay — each
under a stable anchor so brand, plans, and metric definitions can ref them. Persona
names enter `glossary.md` as ubiquitous language: when an agent writes "the indie
dev," it must resolve to one canonical definition.

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
