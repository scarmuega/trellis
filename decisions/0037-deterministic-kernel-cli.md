---
provenance: authored
status: accepted
date: 2026-08-03
---
# 0037 — Deterministic kernel: the `trellis` CLI

## Context
The steward mandate has carried the extraction clause since 0015 ("if parts of
this mandate prove to need determinism … that extraction is the CLI's charter"),
and `spec/runtime.md` staged it as Stage 2, pull-triggered by "a check that must
never be probabilistic." Dogfooding pulled the trigger: the conventions the
members walk as prose have stopped moving. Twenty of the lint checklist's
twenty-four items are pure parse/graph/regex work that an LLM re-derives — and
occasionally fumbles — every ritual; the dispatch scan is already deterministic
but lives as awk one-liners in a workflow; the gate is a regex-over-4KB Node
script; frontmatter lifecycle flips are hand edits an agent can get wrong in
ways no check catches until the next lint; and every generated view the spec
names (plan board, CODEOWNERS, tag indexes, orgchart, open escalations) remains
unimplemented because prose instructions are a poor place to keep a generator.

Three costs recur. Probabilistic enforcement of mechanical rules wastes the
judgment budget: a steward session spends its context re-deriving reference
resolution instead of on the four items that need it. Duplication drifts: root
detection and frontmatter extraction exist in three dialects (JS regex, awk,
prose). And the reference binding stays load-bearing: without a kernel, a
second binding (pi, Codex, OpenCode) must re-implement the mechanical half from
the checklists.

## Decision
**A Rust CLI, `cli/` in this repo, one binary named `trellis`.** It codifies
the deterministic conventions — and only those:

- **Lint**: the mechanical lint items, with findings shaped for verbatim relay
  to owners. Judgment items (2's suspicious-edit call, 7, 16, 23's
  backed-by-the-language nuance) are *reported as skipped*, so the steward's
  LLM pass knows exactly what remains. The split is explicit in the output,
  never silent.
- **Dispatch scan**: the `status: ready` / `awaits:` / `owner:` /
  `complexity:` read, ported from `dispatch.yml`'s awk with identical hold and
  fail-closed semantics. The complexity→model/effort/budget mapping stays the
  workflow's (0032): the CLI ships defaults but the workflow's `--map` flags
  remain the tuning point.
- **Gate**: `trellis gate` speaks the PreToolUse hook protocol and ports
  `gate.mjs` exactly — append-only committed decisions, generated-provenance
  marker check, frontmatter warning, inert outside a root, fail-open always.
- **Lifecycle porcelain**: guarded status flips (`plan release|claim|block|
  unblock|retire`), escalation-record scaffolding under `## Escalations`
  (0036's schema), format-preserving frontmatter edits that refuse rather than
  reflow.
- **Views**: the five specified generators (board, CODEOWNERS, tags, orgchart,
  escalations), pure functions of the tree plus git history, stamped
  `provenance: generated` and `generated-by:` — which makes lint item 2
  mechanical for them (regenerate and diff). Two frontmatter keys are added by
  this decision, both on artifacts the *binding* owns rather than the model:
  `generated-by: trellis view {name}` on generated views (the stamp above), and
  an optional `github:` on `org/{role}/holder/ref.md`, which the CODEOWNERS
  view reads to emit a handle. Neither enters `spec/model.md`: a forge handle
  in the normative core would hardcode a forge into the model, which 0019,
  0024, and 0032 each refused; a role without one is emitted as a comment line
  naming the gap, so the view degrades rather than lies.
- **Derived values**: effective automation class, strategy band, held-vs-
  blocked, dwell — computed once, queryable, instead of re-derived per session.

The binary embeds `template/` and the spec version at build time — a binary
built from a SHA is self-consistent where `${CLAUDE_PLUGIN_ROOT}` does not
exist (CI, other bindings) — with `--plugin-root` as the live-checkout
override. Registries stay instance-read at runtime: plan types and tags from
`conventions.md`, cadences from `rituals.md`; the kernel hardcodes the spec,
never the instance.

**The judgment boundary is unchanged.** Plan effectiveness stays entirely with
`org/focus`; escalation prose, retirement verdicts, and readiness's
underdetermination test stay with their roles. The CLI writes only what a role
invoking it could write; ownership and mandate rules bind the invoker, not the
tool.

`hooks/gate.mjs` is superseded: `hooks.json` prefers `trellis gate` and falls
back to the script until distribution is solved, then the script goes.
Distribution itself is deferred — local builds and `cargo install --git` while
dogfooding (0012's no-version-field stance extends: the binary floats on SHA).

**The kernel changes in lockstep with the model.** A kernel is a second
encoding of rules the prose already states, and a second encoding that may lag
is worse than none: it reports a clean root against last month's conventions,
with the authority of a deterministic check. So a change to the model is not
complete until the kernel carries it, **in the same commit** — a new or edited
lint item, a schema or enum change, a new status, a spec bump and its re-pins.
The kernel is never a follow-up plan; a model change that cannot be translated
yet is not ready to land.

Prose cannot hold that line — this repo's own history is the evidence, the v14
bump having shipped with all three instance pins stale at v13 (0032). So the
obligation is enforced where it can be: `cli/tests/lockstep.rs` asserts the
rule table covers exactly `checks/conventions-lint.md`'s items, the readiness
walk covers exactly `checks/plan-readiness.md`'s, the embedded spec version
matches `spec/model.md` and every shipped instance pin, and the closed enums
still appear in the schema prose. CI triggers on `spec/**` and `checks/**` for
the same reason. The checklists stay the single *source*; the tests only
guarantee the encodings agree, never that the rule is implemented well — that
is what the fixture suite is for.

## Consequences
Rituals get cheaper and sharper: the lint ritual becomes `trellis lint
--format json` plus an LLM pass over the reported judgment remainder; view
refresh becomes `trellis view … --write`. Checks that must never be
probabilistic can now run in CI (`lint`, `view --check`), which this repo
gains for the first time. A second binding inherits the kernel instead of the
checklists. The two shell brains (awk, gate.mjs) stop drifting from the spec
they encode.

The costs accepted: a Rust toolchain enters the repo; every model change now
carries a translation step, which the lockstep rule above makes non-optional
and slower by design — the alternative is a kernel that is confidently wrong;
and until distribution is solved, the gate's Rust path is best-effort on
machines without the binary (the mjs fallback holds the guarantee meanwhile).

The lockstep tests bound drift, they do not eliminate it. They compare *item
sets*, not semantics: an item whose wording changes while its number stays put
passes them, and no test can tell that a rule's implementation still means what
the prose says. The residual risk sits exactly there, and the mitigation is the
per-item fixture suite plus reading the diff — not a green check.

## Alternatives rejected
**Keep extending the shell tooling** (more awk, more gate.mjs) — the dialects
were already drifting, and neither is testable against fixtures.
**A separate repo** — spec and enforcement would version independently; the
repo's own rhythm (decision → member → check, in one commit) argues for
lockstep.
**TypeScript/Node kernel** — inherits a runtime the non-Claude bindings don't
carry; the gate's latency budget and single-binary distribution favor Rust.
**Hardcoding the complexity→session mapping in the binary** — moves 0032's
tuning point into a rebuild; defaults-plus-`--map` keeps the workflow the
binding's knob.
**Mechanizing the judgment items** (glossary coverage, technology-free founding
map, suspicious edits) — pattern-matching them would produce confident false
verdicts; the split-and-report design keeps them owned by roles that can
actually judge.
**Letting the kernel lag the model, caught up on a cadence** — a steward ritual
or a "kernel catch-up" plan reconciling the two. Rejected: it inverts the
guarantee. Between the model change and the catch-up, `trellis lint` reports
clean against conventions the root no longer has, and every consumer that
trusts a deterministic check is misled precisely because it looks
authoritative. Lag is tolerable in a report and not in a gate.
**Generating the rules from the checklist prose** — no drift by construction,
and briefly tempting because the items are numbered and regular. Rejected: it
would make the checklist a DSL, and the items are English arguments carrying
rationale, escalation targets, and named exemptions (item 24's held plan, item
23's `customer-supplier` no-op). Compiling that would either lose the argument
or turn the prose into code with the argument in comments — the checklist is
read by humans and agents far more often than by the kernel.
