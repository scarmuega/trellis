---
provenance: authored
status: accepted
date: 2026-07-30
---
# 0032 — Plan complexity: reasoning depth as a dispatch signal

## Context
Plan dispatch (0029) runs every `ready` plan through one session shape: one
model, one effort level, one `--max-budget-usd 5`. The reference workflow's own
trailing comment already concedes the gap — *implementation costs more than an
artifact edit* — but a plan has no way to say so. The obstruction is
deliberate: 0019 and 0024 keep provider facts out of the domain root, so a plan
cannot name a model, an effort level, or a price without dragging harness
vocabulary into artifacts the framework promises are harness-neutral.

What is missing is not the knob — the binding already has one — but a
**provider-agnostic property of the work** the binding can map to it. Nothing in
the root models complexity, capability tiers, or reasoning depth today.

## Decision
Add an optional plan field **`complexity: mechanical | standard | deep`** — a
closed ordinal the spec defines by observable task properties: the reasoning
depth the work demands of its taker, never its size. A large migration can still
be `mechanical`.

- `mechanical` — the plan fully determines the change; execution is
  transcription and verification is cheap and already exists.
- `standard` — local design judgment within the declared `contexts:`; the plan
  fixes outcomes, not construction.
- `deep` — novel design or cross-context judgment with a wide blast radius; many
  defensible constructions deliver the same outcome.

The set is closed and lives in the spec, like `status` and unlike `type`. `type`
is an open registry because it is descriptive taxonomy nothing branches on;
`complexity` is behavioral — the runtime branches on it, and the flagship taker
(`trellis:coder`, 0031) is plugin-shipped and portable, so plan-readiness's "one
bar, checked twice" requires owner and taker to mean the same thing by a tier in
every root. This is the whole normative change (a schema field plus its
semantics in `spec/model.md`); no Rule references plan frontmatter fields, so
none changes. Spec **v14 → v15, additive** — existing plans stay valid; with the
field absent they dispatch at the binding's stated default.

"The binding's default" is a value the binding **states**, never one it inherits.
The reference workflow names a model and an effort level on every branch,
including the absent one, because an omitted `--model` silently picks up whatever
`model` setting the runner's own configuration carries — so the same plan, at the
same commit, would dispatch on a different model depending on who ran it. That
violates the instance boundary guarantee that an agent's behavior is attributable
to a specific commit plus specific pins, and it fails in the expensive direction:
an operator whose personal default is the strongest model silently funds every
unscoped plan at that tier.

The reference binding states **`standard`** as that default (an instance may
choose otherwise; the spec fixes the tiers, not the mapping). That tier is the
unmarked case — the plan
fixes outcomes and the taker exercises local design judgment — so the other two
are deliberate departures from it: `mechanical` claims the plan fully determines
the change, `deep` claims novel cross-context judgment. An unscoped plan is the
ordinary one, and the schema follows the same shape as `relation:`, whose absent
value is likewise the common semantic rather than a degraded one.

**Specificity returns in the binding**, where 0019 confines lock-in: the
reference `dispatch.yml` maps each tier to a concrete `--effort`, `--model`, and
`--max-budget-usd`, and `conventions.md`'s Runtime binding section names that
mapping's home. The shape of the reference mapping is itself evidence the
semantics are right — every tier gets a strong model and the first knob the
binding turns is reasoning effort, not model class; only the top tier escalates
the model. Nothing in it is graded by diff size.

Two behavioral clauses keep the field honest rather than decorative. The coder
treats a **mis-scoped tier as a blocker** — work marked `mechanical` that turns
out to demand design judgment is an under-provisioned session, escalated through
the existing blocker path, never pushed through. And the coder **may propose a
tier on its residue draft**: it just implemented the adjacent code and is best
placed to estimate, and residue is always `draft`, so release remains the
owner's act and the owner ratifies or amends.

Readiness is untouched. `deep` is not underdetermined: plan-readiness item 9
tests whether two readings would deliver the business *different things*, while
`deep` means many constructions deliver the *same* thing. Conventions-lint item 4
validates the enum with the established "(if present)" idiom.

## Consequences
Dispatch sizes a session to the work instead of to a constant, and the domain
root still contains no model name, effort level, or price — an instance retunes
the mapping in one `case` statement without touching a plan or the spec. A
mis-tiered plan is observable and cheap to challenge (the taker escalates), which
is the property `progress:` lacked when 0030 rejected it.

The spec bump also catches up the instance pins — `template/decisions/0000-adopt-trellis.md`
and the eval skeleton's two — which all sat at v13 and had already missed the v14
bump, so a freshly scaffolded root failed conventions-lint item 18 on arrival and
`checks/plan-effectiveness.md`'s dispatch group read the skeleton as pre-`ready`.
All three now read v15.

The eval fixtures stay green: the field is optional, no fixture declares it, and
`evals/grade.mjs` parses no plan frontmatter.

## Alternatives rejected
**A `model:` or `budget:` field** — provider facts inside the root, exactly what
0019 confines to adapter surfaces; it would also stale on every model release.
**An open registry like `type:`** — the portable coder must read one meaning in
every root, and an instance-defined set would leave the tiers with no definition
but the instance's own mapping table, collapsing the spec/binding seam this
decision exists to preserve. **Two dimensions, depth × volume** — volume already
has a repo-native answer in plan decomposition (0026), and a size field would
institutionalize the monolith plans that pattern exists to split; the failure
modes are also asymmetric, since an under-budgeted session halts visibly
(plan-effectiveness item 18 already names budget exhaustion) while an
under-modeled one quietly produces a plausible wrong implementation. **A new
plan-readiness item** — mis-tiering is not underspecification; the plan is still
ready and dispatchable, so the checklist has nothing to say about it. The taker's
existing blocker path already covers the case that matters. **Solver-relative naming**
(`easy|hard`, `difficulty`, `effort`) — 0030's unfalsifiability standard; these
tiers anchor to task properties a reviewer can check, and `effort` is a literal
provider API knob besides.
