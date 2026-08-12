---
provenance: authored
status: accepted
date: 2026-08-12
supersedes: [0031]
---
# 0047 — Implementation holders are domain-local

## Context

0031 shipped `agents/coder.md` (`trellis:coder`) as the portable implementation
holder: a domain whose plans land in code adopted it with a local mandate plus a
`holder/ref.md` to the agent. The premise was that implementation discipline is
generic — gate, implement, PR, residue — so the identity could be shared the way
the steward's and focus's are.

Dogfooding said otherwise. The first production adoption (the TxPipe domain)
needed a mandate-wins override in its ref from day one — the agent's generic
posture treated the domain's vendored `solution/{bc}/codebase` submodules as
"another repository" to block on, when they are precisely its implementation
surface — and was then replaced outright by an inlined `holder/system.md`
package carrying the domain's own boundary, surfaces, and idioms. The
steward and focus stay portable because they operate the *model*, which is the
same in every domain. An implementation role operates the *business*: which
repositories are writable, where the org boundary runs, what verification
means, how much of the readiness bar the taker re-checks — every one of those
is domain policy, and a portable agent can only carry it as generic posture
that each domain overrides in prose. The override file was becoming the real
holder; the agent was becoming indirection.

## Decision

The plugin ships no implementation agent. `agents/coder.md` is deleted; each
domain authors its implementation holder ad hoc as a domain-local package —
`org/{role}/holder/system.md` plus at least one eval (lint item 6), exactly the
"Add a role" procedure every other agent-held role already follows. Dispatch
needs no change: it routes by the plan's `owner:` to whatever holder the role
declares, and the package branch of `act` binds it.

What 0031 got right survives as documented discipline rather than shipped
identity: the runtime companion's "Implementation holders" section and the
conventions skill's "Adopt an implementation role" procedure state the
invariants a package should carry — block on a specification gap rather than
guess, deliver code as a PR (draft while unfinished, never self-merged), file
residue as a `draft` plan the owner releases. `checks/plan-readiness.md`
remains the canonical bar with `/trellis:plan` as its release-side consumer;
how much of it a taker re-checks at pickup is now the domain's call, made in
its own holder.

## Consequences

A code-bearing domain now writes a holder package instead of pointing a ref at
the plugin — more authoring up front, in exchange for instructions that are the
domain's actual policy rather than generic posture plus overrides. Pinned
holders no longer auto-track plugin improvements; portable fixes to
implementation discipline travel as spec and skill text that domains adopt
deliberately. Lint item 6 gives every such package an eval floor. 0031's
remaining consumers — decisions 0032, 0033, 0035, 0036 cite the coder's
behavioral clauses — read those clauses as obligations of whatever
implementation holder a domain binds; the clauses themselves stand.
