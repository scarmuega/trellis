---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0018 — Premises describe the world; rules oblige the operator

## Context
The v11 rewrite (0013) moved worldview and machinery in one gesture, and the
rationale began tracking rules: premise 2 carried rule 11's attributability
obligation, rule 11 carried the pivot-asymmetry worldview, and premise grounds
cited rule numbers, lint checks, and enforcement rituals ("rule 11, orphan lint,
the derivation sweep"). The rationale's job is the why of the framework — the
premises under which operating this way makes sense — not a shadow copy of the
how.

## Decision
Two buckets, strictly assigned. Premises are claims about the world,
imperative-free — the only "must" they may carry is descriptive necessity ("the
search must sustain itself economically"), never an obligation on an operator.
Rules are obligations — how to operate under the premises; a one-line
justification is permitted, standing worldview is not. Grounds name structural
elements (directories, files, frontmatter fields, named mechanisms) — never rule
numbers, lint items, or enforcement rituals: rules answer to premises, not the
reverse. Applied: premise 2 sheds the attributability obligation to rule 11 and
absorbs the change-asymmetry worldview from rule 11's tail; premise 1 drops
"versioned" (mechanism vocabulary — the worldview is "chosen and revocable");
premises 2–3 regrounded on elements. The corollary now states the misfiling
test.

## Consequences
The deletion corollary gains a second audit: an obligation in a premise, or
worldview in a rule, is an editorial bug findable by reading. Operational
definitions (effective-policy derivation, status semantics) stay canonical in
schema comments; playbooks stay patterns. 0013's enumeration stands — still ten
premises; sentences moved between buckets, none were added or removed. Ships as
a follow-up commit under spec v11: nothing normative changed, only filing.

## Alternatives rejected
A separate "Model"/"Concepts" section for definitional content — schema comments
already hold operational definitions canonically, and a third top-level bucket
would give future misfiled sentences a place to hide. Renumbering to fewer,
purer premises — supersedes 0013's enumeration for no semantic gain.
