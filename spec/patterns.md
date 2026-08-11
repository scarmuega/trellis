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

### Decision hatching
A decision is memory, not a manual (spec rule 6): the trail explains why, and
the trail only grows — append-only, never archived (spec rule 13) — so nothing
an operator needs day to day may require crossing it. What keeps the pile from
becoming required reading is where the guidance lives. Standing guidance a
decision establishes hatches, in the same change that accepts it, into the
artifact that governs the behavior: a convention deviation or schema choice
into `conventions.md`; a bounded context's commitment — a runtime, a vendor, an
architecture — into that context's README or, once machine-checkable, its
`contracts/` (see Requirements: prose is the larval form); a change of
authority into the mandate it changes; a standing process into a `rituals.md`
row; a term of art into `glossary.md`. The governing artifact cites the
decision — `(decision NNNN)` — which inverts the lookup: operation reads the
guidance where reading already happens and finds a footnote, instead of
excavating the trail for the current rule. A decision that establishes nothing
standing — the record of a one-shot act — says so in its own Consequences:
`Standing guidance: none.`

Supersession is structural, never an edit: the successor's `supersedes:` names
every decision it fully replaces (spec schema), so the live set is computable
while the superseded file never changes; partial influence stays prose. The
lint computes the rest (checklist item 26): an accepted, unsuperseded decision
unreachable from the operative record — cited by no live authored artifact,
not declared inert, absent from the decision registry in `conventions.md` — is
the finding, and the derivation sweep escalates it to its owner as orphaned
operative content: hatch it, supersede it, or declare it inert. A domain
arriving with a pile compacts once: the owner walks it newest to oldest,
hatches what still operates, records prose-era supersessions and inert
classifications in the registry — judgment made once, not re-derived every
sweep — and one decision records the codification.

The shape deliberately not chosen is consolidation inside the pile: a
restatement decision that supersedes a set and restates its guidance freezes on
acceptance like any other, so its first correction forces another wholesale
copy — append-only digests re-accumulate — and it plants operative guidance in
the one kind nobody should have to read. The editable, owned guidance artifact
gets the same consolidation with git history as its trail. `supersedes:`
remains what it is everywhere else: the mark of a choice genuinely replaced.

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

### Automation shapes
Every automation resolves to an existing kind by who triggers it and what
relation it bears to judgment. A deliberate action someone invokes with inputs
is an `act` invocation — its spelling (a slash command, a CLI verb) is the
binding's adapter surface, outside the root (spec rules 1–2). Know-how that
should activate when a situation is recognized is a skill, placed per
Cross-cutting procedures. A worker with identity and bounded authority is a
role — mandate plus holder; an agent is a holder form, never a shape of its
own. A standing cadence is a `rituals.md` row. An invariant needing no
judgment belongs to the gate, never to a prompt-borne member — which
enforcement is deterministic is the binding's honest declaration (runtime
companion). The naming rule carries the test: a verb is an invocation, a topic
is a skill, a role is a holder.

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

### Plan decomposition
A large effort wants an umbrella and independently workable pieces, and both are
plans — the same kind, so no directory is earned (spec rule 7). Never
`plans/{plan}/` with an index file: a plan-family folder is a grouping, and the
index inside it is an artifact with no schema. The shape is flat: each piece is
a sibling `plans/{parent}-{piece}.md` with full plan frontmatter — its own
`status:`, its own refs — activated, blocked, and retired independently; the
umbrella is a plan like any other (`type: initiative`, or a type the instance
registers) whose body carries the decomposition as an anchored section linking
each piece and the seams between them. A seam that is an ordering is declared,
not narrated: the later piece carries an `awaits:` edge naming the earlier
(spec schema), so dispatch holds it — still `ready`, undispatched — until its
sibling retires; the family releases at once and drains in dependency order,
the sequence machine-visible to the scan instead of living in prose the scan
cannot read. Family membership is a registered tag on
every member; any roll-up is a generated view over the tag, never an authored
hierarchy. Prefix naming keeps the family adjacent in a flat listing (spec
rule 10). The effectiveness walk needs no special case: each piece answers for
its own worth, and the umbrella — holding the shared metric refs — answers for
the whole. A family that starts wanting its own goals, metric definitions, and
plans about its plans is spec rule 8 firing: that's a domain — give it a root,
never a deeper tree (see Stratified alternation).

### Execution health
One metric family is portable across every domain, because it reads the machine
rather than the business: whether the plans that exist are moving. It computes
from `plans/` frontmatter and git history alone — no domain knowledge, no
judgment — so one generator serves any root, and a domain has it on day one,
before it knows which business metrics it will own. Three readings carry it. The
*census* is the stock: plans per `status:` — draft, ready, active, blocked,
retired — cut by `type:`, by `owner:`, and by the automation class of the
subdomains each touches. The *flow* is what a census hides, and it comes from the
history of `plans/`: entries authored, releases (`draft → ready|active`), and
closures per cadence, plus *dwell* — how long each plan has held its current
status, counted from the commit that set it. The *mix* is derived: shares of the
census, work in progress per owner, and cycle time as WIP over closure rate — the
honest form of "how long a plan takes here."

Two tempting readings need refining before they mislead. Percent-complete is not
one of the measures: no progress field exists, and none should be added —
self-reported completion is unfalsifiable, and its failure mode is the plan that
is ninety percent done for a quarter. The mechanical substitutes are dwell and
*movement*: an active plan whose artifact and declared contexts saw no commit
within a cadence is not progressing, whatever its body claims. A domain wanting
finer resolution decomposes (see Plan decomposition) — a family of six siblings
with four retired is a percentage, and one the filesystem computes. Fulfillment
rate needs the same care: `retired` conflates shipped, abandoned, and superseded,
so closure rate measures clearance, not success. Read it as clearance, or make
the split mechanical with a body convention — an anchored verdict section
recorded as the status flips (shipped | abandoned | superseded), which costs no
schema change and is what a plan's owed verdict looks like once written down.

These numbers are instrumentation, not goals, and the distinction is structural.
`metrics/definitions.md` holds what the business is trying to move: each
definition carries a target, an owner, and a plan that refs it. Execution health
has none of the three and should acquire none — it belongs where generated
readings land, a plan board under `metrics/actuals/` with `provenance:
generated`, refreshed on the sweep cadence and stale past it like any other
reading (spec rule 5), written by the steward, which already keeps generated
views. No board directory is earned: a view over the plan set is a view (spec
rule 7). The reason for the discipline is that a definition earns a target, a
target earns a plan, and a plan whose purpose is to improve plan throughput is
either Goodhart with a mandate — pieces split to raise the closure count, drafts
withheld to keep the blocked share low, plans retired to clear the board — or
spec rule 8 firing, since a unit needing plans about its plans is a domain.
Thresholds are the right form for these numbers: a dwell figure that says *look*,
never a number anyone is graded against.

What the board buys is that the mechanical fraction of the effectiveness walk
stops being re-derived every cadence: ready dwell past the dispatch cadence is
the stalled queue, blocked dwell past a ritual cadence is the stalled blocker,
active dwell past a plan's measurement horizon is the owed verdict, and WIP
sitting on generic or low-ranked subdomains is the attention-allocation question
in one cut — each arriving as a sorted queue, so the walk's judgment spends
itself on plans the arithmetic already ranked. The mix reads against the domain's
own shape: draft-heavy means the domain authors faster than it executes,
ready-heavy indicts the dispatcher rather than the owners, blocked-heavy means
the escalation channel is not clearing. Across roots the readings mean the same
thing everywhere, so a portfolio can roll its ventures' boards up — by spec rule
1, how the numbers get there is the consumer's tooling. The limit is the one no
instrument sees past: execution health can be perfect while the business dies —
every plan moving, closing, on cadence, against strategies that do not matter. It
answers whether the machine runs, never whether it runs on anything worth
running on; that question stays with the effectiveness walk's challenges and the
maturity ladder. A board trending up while outcome metrics flatten is not a
contradiction to reconcile — it is the value-dead finding arriving early.

### Loop observability
Every standing loop in a domain — a ritual on its cadence, the dispatch tick,
an agent session retried until it lands — keeps its exit and progress
conditions in artifacts and git, never in an agent's memory or its self-report.
Agents are stateless (rationale premise 9): whatever a loop needs to stop,
skip, or resume must be reconstructible from the tree; and execution must carry
status (premise 5): a loop whose progress lives in a transcript has none. The
shape is already everywhere the runtime works. Dispatch is idempotent because
the claim is an artifact edit — the taker's `ready → active` flip is what
removes a plan from the scan, so a dead session changes nothing and the next
tick retries. A hold is a status read — `awaits:` targets unretired — and its
release is another. The readiness gate ends a doomed loop by flipping
`ready → blocked`: a declared field, not a memory that this plan was tried. What
*clears* that block is filed the same way — an escalation record in the plan
itself, so the question a human must answer is reconstructible from a checkout
rather than from a forge thread the next session cannot see.
Progress is dwell and movement (Execution health) — commits touching the plan
and its declared contexts, never a percentage anyone typed. The failure mode is
the loop whose termination lives in a report: a session that claimed a plan in
its transcript but not on the default branch is dispatched again forever
(plan-effectiveness item 19), and a ritual that ran without a trail did not
run. The test for any standing loop: erase every participant's memory between
ticks — does it still stop, hold, and resume correctly from the tree alone? If
not, its state is filed in the wrong place.

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

### Strategy maturity
A strategy's `status:` is the maturity ladder — the record of what the evidence
has shown, never what anyone hopes: `raw | defined | validated | implemented |
established | discarded`. Each stage names the work the strategy owes and the
metrics that read it. `raw` owes refinement — sharpen `need:` and
`differentiation:` until they are falsifiable; work on the artifact itself,
which no plan carries. `defined` owes validation: an `experiment` plan with a
decision criterion, read through learning and leading indicators. `validated`
owes implementation — this is the commitment line, where the strategy starts
inducing subdomains (spec rule 11): initiatives, read through delivery
progress. `implemented` owes monitoring — the metric sweep does the work:
outcome metrics against their `definitions.md` targets. `established` owes
hardening and optimization, read through efficiency, unit economics, and
reliability. `discarded` owes nothing — failed, unmeritorious, or superseded
(see Pivot) — and any plan still advancing it has outlived its reason.

The coarse vocabulary survives as bands, and prose using it stays true:
aspirational is `raw|defined`; committed is `validated|implemented|established`
— only the committed band induces; retired is `discarded`. The ladder advances
only on evidence, never on ambition: each move is its owner's authored edit
citing what changed. Skipping stages is legitimate when the evidence is real —
a business selling before validating has run the strongest experiment there
is; record the stage the evidence supports. The effectiveness walk reads the
stage, not just the band: every strategy past `raw` enters it — an unvalidated
strategy claiming attention has announced a validation debt — and the stage
names the plan the walk should find and the metrics that count as evidence.
The test for any status edit: what evidence changed? An edit justified by
ambition alone is misfiled.

### Economic lineage
Strategies decompose by need and differentiation, not by revenue stream — so a
portfolio routinely splits producing value from capturing it. Open-source
devtools are the canonical case: one strategy builds adoption, installed base,
and catalog content while grants, hosting, or paid trust services collect the
revenue. Without an edge, the producer looks economically self-justifying and
the dependency stays invisible. `funded-by:` edges (spec rule 12) are the
machine-readable skeleton: each committed strategy declares what sustains it —
itself (`self`), a sibling strategy, or an external ref. The value-capture
strategy is a genuine strategy, never bookkeeping: its `need:` is the payer's
need — a grants strategy fulfills a funder's need for a maintained ecosystem; a
hosting strategy, a user's need for managed infrastructure. `relation: current`
records the operating model as committed; `relation: intended` records a
conversion thesis — grants today, hosting tomorrow — and one strategy carries
both edges at once. The division of labor is strict: the edge carries topology
(who captures), the maturity ladder says how real the operation is, metrics say
how capture performs, and `economics.md` narrates pricing and unit economics
over the skeleton — never amounts or split ratios on the edge; that precision
belongs to the narrative and the metric definitions. The failure mode the edges
expose is portfolio-level: a funding strategy dying before the conversion
lands. The lint catches the static forms — a committed strategy nothing
sustains (economic orphan), a committed band funded only by itself in a circle
(no capture point); the derivation sweep flags the dependents of a discarded
funder exactly as it flags induced subdomains; and the focus walk weighs
attention on producers against the health of whatever captures their value.

### Pivot
A pivot is a strategy edit, not a rewrite of the world. The founding map endures —
if it doesn't, that wasn't a pivot but a different business. Procedure: set the old
strategy's `status: discarded`; commit the new one, `supersedes:` naming the old.
Every subdomain the retired strategy induced is now orphaned (spec rule 11): the
derivation sweep flags each for its owner to re-parent onto a surviving commitment
(often with a different `class:` — classification is per edge) or archive. Its
funding dependents orphan the same way (spec rule 12): every strategy whose
sustaining `funded-by:` edges pointed at the discarded one is flagged for its
owner to re-fund onto a surviving capture, commit its conversion thesis, or
reconsider its own commitment. Orphans
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
