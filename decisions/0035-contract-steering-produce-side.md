---
provenance: authored
status: accepted
date: 2026-08-01
---
# 0035 — Contract steering: wiring the produce side of published language

## Context
Dogfooding evidence: roughly fifty plans authored and completed, zero
bounded-context contracts involved — not because contracts lacked value, but
because no automation ever asked the question. The inventory confirms it:
contracts are the only load-bearing kind in the hierarchy whose loop was never
closed. Every other kind earned production and consumption edges from some
decision — metrics get the sweep, plans get dispatch and the effectiveness
walk, decisions get the gate, subdomains get the derivation sweep, `funded-by`
got items 16–17, `awaits` arrived with its own consumers (0033). Contracts got
a directory, placement rows, the Requirements pattern's hatch pressure
("prose is the larval form") — non-normative prose no loop executes — and
exactly one consumption instruction: the coder reads and satisfies them, and
escalates when one would have to change. **No checklist item, ritual, or
authoring step anywhere triggers producing one.** Held to the bar the model
applies to its own schema fields (0026: consumed by nothing in the walks),
contracts as wired sat below it; fifty plans routing around them is the system
behaving as built.

Part of the sample is discountable — a mostly single-context domain is
seam-poor, and a contract earns nothing inside one language boundary — but the
discount does not touch the finding: the plans that did cross seams had no
automation asking either. Readiness item 11 even listed "a `contracts/`
artifact" among its verification options, but in a code context "a test suite"
always satisfies the item first, so the taker never reaches past it.

## Decision
Close the produce side with steering at every point where work touches a seam,
and two standing pulls so the steering stays honest without anyone
remembering. All non-normative — agent prompts, command steps, checklist
items; no schema, no Rule, spec stays v16.

**Readiness item 11 gains the interface clause** — the highest-leverage hook,
because it is the "one bar, checked twice" surface: the owner meets it at the
release offer and the coder re-checks it at pickup. An interface-shaped done
criterion — output another context, party, or system will consume — is
verified by its contract: an existing one, or one the plan hatches. Tests on
one side of a seam do not verify the seam; an interface deliverable with no
contract is verification that does not exist.

**The coder produces, not just satisfies.** A new implement rule — work that
stabilizes an interface delivers its contract into the context's `contracts/`
as part of the PR; prose in a README is the larval form, never the
deliverable — and "an interface left in prose" joins the residue examples.

**`/trellis:plan` reads and asks.** Step 5's research list gains the touched
contexts' `contracts/` (the one thing in a bounded context that is published
language was absent from the read list); step 6 asks the interface question at
drafting: a deliverable another party consumes names its contract in the body,
existing or to-be-hatched.

**Lint item 23** — the supply-side pull, checking each relation's claim
against its artifact. Every relation crosses a language; the annotation
determines who owes the artifact and where it can live. `published-language`
means this root publishes: the publishing context's `contracts/` holds it —
nothing behind it is a promise with no artifact, consumers conforming to
conversation. `conformist` and `acl` claim an upstream language this root does
not own and cannot hold (rules 1–2): the relation carries a pinned ref to what
is conformed to or translated — conforming to something unnamed is the same
failure inverted. `customer-supplier` claims negotiation, not an artifact, so
nothing fires: its interface may still be larval, and forcing a contract at
annotation time is the premature hatch the trigger discipline exists to
prevent. The template's context map documents the three cases at the point of
annotation.

**The derivation sweep watches contracts** — the payoff that makes producing
one worth it: commits touching a context's `contracts/` escalate every context
the map relates to the publisher, because the context map is the consumer
registry. Published language changing under its consumers is exactly the
upstream-edit-orphans-downstream shape the sweep already handles for
strategies. The rituals row and the conventions skill (lint summary, plus a
"Hatch a contract" procedure making the move citable) update to match.

**Trigger discipline** is the constraint throughout: every clause fires on
seams — interface-shaped done criteria, published-language annotations,
contract edits — never on plans generally. A blanket "produce contracts"
instruction would recreate the accumulation problem the Requirements pattern
exists to warn against.

Eval fixtures stay green: no fixture's context map annotates any relation
(the four annotations appear only in the template's placeholder option list),
the focus evals grade effectiveness findings rather than lint, and readiness
items are not graded at all.

## Consequences
Contracts acquire producers to match their one consumer, and — through the
sweep — a reason to exist that pays the author back: an interface in
`contracts/` notifies its consumers when it changes; the same interface in
prose does not. The interface question is now asked four times along a plan's
life (drafting, release, pickup, delivery), each by the party best placed to
answer it, and each firing only when the work is seam-shaped. Whether this
moves the count from zero is measurable the same way the gap was: the next
fifty plans.

## Alternatives rejected
**A `contracts:` ref list on plan frontmatter** — no consumer branches on it:
the coder already reads every declared context's `contracts/`, and body refs
serve the narrower pointer. The 0026 bar stands; promotion waits for a
consumer.
**An effectiveness item for seam-crossing plans without a contract**
(a `risk` addressed to both contexts' owners) — deferred by pull, not
rejected: the dogfooding domain is seam-poor, so the item would grade
territory the evidence has not reached. Pull-trigger: seam-crossing plans
recurring in dogfooding, or one integration failure a contract would have
caught.
**A contracts-per-context rule** ("every bounded context declares its
published language") — ceremony: most contexts publish nothing and owe
nothing; the map's annotation already declares who does.
**Extending item 23 to `customer-supplier`** — the annotation claims a power
relationship, not an artifact; an internal interface may legitimately run
larval until work touches it, and "stabilized" is a judgment lint cannot make.
The challenge belongs to the deferred effectiveness item; the hatch belongs to
the seam steering.
**Linting anchored requirements for staleness** ("`## R-` prose older than N
with no hatch") — "stabilized" is a judgment no pattern-match can make; the
unfalsifiability standard (0030) applies to lint items too.
