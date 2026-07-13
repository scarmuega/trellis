---
provenance: authored
status: accepted
date: 2026-07-13
---
# 0021 — Skills have exactly two homes; the root is not one of them

## Context
The hierarchy places skills in two sites: `solution/{bc}/skills/{skill}/`
("ALL procedures live here") and `org/{role}/holder/skills/` (part of a local
agent package). The question arose whether a domain should also allow a
root-level `skills/` — or, pulling the other way, whether skills should exist
only inside org roles. The answer was implicit in the hierarchy but never
argued through, and nothing named the pressure that produces root junk
drawers.

## Decision
Neither. Skills keep exactly two homes with distinct meanings:

- `solution/{bc}/skills/` holds business procedures — domain knowledge,
  written in the bounded context's language, governed by the automation
  policy through the context's subdomain edges.
- `org/{role}/holder/skills/` holds identity-bound technique — part of an
  agent package, how this particular holder works; never the home of a
  business procedure.

A root-level `skills/` is a grouping, not a kind (rule 7), and is prohibited.
A procedure that seems to belong to no context resolves within the existing
shape: a convention (`conventions.md`), a standing process (a `rituals.md`
row pointing at a context's skill), a runtime concern (binding skills live
outside the root, rules 1–2), or a bounded context the pressure has just
revealed. Codified as the Cross-cutting procedures pattern; lint item 11
names the root `skills/` case explicitly.

## Consequences
"Where do procedures live?" keeps a single answer, per premise 9. Every
business procedure inherits an effective automation class through its
context's subdomain edges — a root skill would have no derivation chain and
no class, a structural orphan the policy cannot reach. The urge to create a
root `skills/` becomes a modeling signal — most often an operations-flavored
bounded context waiting to be named — instead of being absorbed by a junk
drawer. The plugin repo's own root `skills/` is unaffected: that is Claude
plugin layout (decision 0009), not a Trellis domain instance.

## Alternatives rejected
A root-level `skills/` directory — splits the skills mechanism exactly as
"functions as a bag of procedures" did (rejected in 0004): every playbook
gets a lazy home and a correct one. Reachability is no argument: the session
plane makes skills invocable at the root wherever they live in the tree
(`spec/runtime.md`); placement is knowledge classification, not availability.

Skills only inside org roles — `holder/` is identity, not knowledge: holder
skills leave with the holder, so business procedures kept there would not
survive a holder swap (agent → human, agent v1 → v2), defeating the
mandate/holder split; human holders (`ref.md`) have no skills directory at
all, leaving human-executed playbooks homeless against the one-schema
premise; and procedures shared across roles would duplicate instead of having
one home any mandated role in scope can execute.
