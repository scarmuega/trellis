# Trellis — rationale

> The premises Trellis rests on: how the world is, and why the framework follows.
> These ground the Rules in `spec/model.md`; every element of the framework answers
> to a premise here (see the corollary). The specification proper is `spec/model.md`.

1. **Needs are primitive; a business is a search for a viable strategy to fulfill
   them.** Needs exist in the market independently of any business — external,
   slow-changing, technology-free: the model's only invariant layer. A strategy is
   a committed solution to a need — the business model as actually operated, not
   as aspired to. Only commitments induce consequences; intentions do not. The
   search is not optimization of a fixed objective: strategy is chosen and
   revocable, and the search must sustain itself economically to continue —
   viability is knowledge in its own right.
   *Grounds: `market.md` (the founding map), `strategy/` and its status
   lifecycle, `economics.md`, `brand.md`.*

2. **The domain is induced, not found.** `need × strategy → domain`: the domain is
   everything the business must become expert at *because of how* it chose to
   fulfill the need. Aircraft maintenance is in an airline's domain because the
   strategy put it there — an induced subdomain owes its existence to a
   commitment, and to nothing else. Induction also sets the asymmetry of change:
   needs edits are rare and cascade everywhere; strategy edits are the normal
   unit of pivot.
   *Grounds: the founding map (`market.md`) inducing the subdomains under
   `problem/`, `induced-by:` edges, `supersedes:` on `strategy/`.*

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
