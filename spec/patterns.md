# Trellis — patterns & practices companion

> Recurring content that lives *inside* the structure defined by `spec/model.md`.
> Non-normative: descriptions of practice, not rules — adopt, adapt, or ignore per
> domain. If one acquires independent lifecycle in your instance, spec rule 7 tells
> you when it earns promotion.

### Business functions
Marketing, sales, support, finance are not a branch — they dissolve into the
ontology. The function's problem is a subdomain, classified core/supporting/generic
per strategy edge, which is what puts business functions under the automation
policy. Its solution is a bounded context. Its campaigns and initiatives are plans.
Its standing processes are rituals. Its ownership is a mandate. "Everything mktg"
is a generated view over tags, per spec rule 7.

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

### Cross-cutting procedures
Skills have exactly two homes. `solution/{bc}/skills/` holds business
procedures — domain knowledge written in the bounded context's language,
governed by the automation policy through the context's subdomain edges.
`org/{role}/holder/skills/` holds identity-bound technique that travels with
an agent holder — never a business procedure, which must survive holder swaps.
There is no third, root-level home: a root `skills/` is a grouping, not a kind
(spec rule 7), and sits outside the derivation that gives every procedure its
automation class. A procedure that seems to belong to no context is a modeling
signal, and it resolves within the existing shape: a convention
(`conventions.md`), a standing process (a `rituals.md` row pointing at a
context's skill), a runtime concern (binding skills live outside the root,
spec rules 1–2), or — most often — a bounded context waiting to be named: a domain
accumulating "general" runbooks has an operations context it hasn't admitted
to. Reachability never argues for the root — the session plane makes skills
invocable at the root wherever they live; placement is knowledge
classification, not availability.

### Personas & market
Personas are founding-map knowledge — demand-side and technology-free, so they
endure pivots: sections within `market.md` — who they are, needs, context,
willingness to pay — each under a stable anchor so brand, plans, and metric
definitions can ref them. A persona's needs connect to the map's `## N-{slug}` need
anchors. Persona names enter `glossary.md` as ubiquitous language: when an agent
writes "the indie dev," it must resolve to one canonical definition.

Market splits across the existing spine rather than getting a directory: segmentation
and competitive landscape are founding-map knowledge (body of `market.md`); sizing and
pricing-per-segment are `economics.md` content. `market.md` is one authored document,
not a "market" directory — the latter would be a grouping, not a kind.

### User journeys
An as-is journey — how a persona currently experiences the problem, where it hurts —
is discovery knowledge that belongs to the persona: a section next to their
definition in `market.md`, anchor-referenceable. A to-be journey is a goal, and goals already have a
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
Every subdomain the retired strategy induced is now orphaned (spec rule 11): the
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
metrics, and plans, spec rule 8 fires: it becomes a new domain root, where the full
machinery applies again. Recursion happens via fractal roots (spec rule 9), never
deeper trees.

### Classification inversion (portfolio)
Across ventures sharing a substrate, check each generic subdomain consumed by
venture A for being a core subdomain of some venture B — in the portfolio or the
market. The inversion is healthy and expected: it marks a clean factoring boundary
and a candidate internal interface (mandate-governed agent payments: one venture's
core, a sibling's generic — same problem, opposite classification, two doors
down). Its absence is the smell: ventures duplicating each other's core, or one
accidentally building a sibling's product as "internal tooling." This check runs
at a portfolio root (spec rule 9: a portfolio is just a domain) over its venture refs;
how classifications reach that root is the consumer's tooling, per spec rule 1.
