# Changelog

All notable changes to the Trellis plugin are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

**Versioning.** While Trellis is single-maintainer and pre-1.0, the plugin omits an
explicit `version` and floats on the git commit SHA — every push to `main` is a new
version for installers (see `decisions/0012-plugin-versioning-float-on-sha.md`).
Once there are external consumers this switches to explicit
[Semantic Versioning](https://semver.org/): each release gets a `## [x.y.z]` section
here and a matching `vx.y.z` git tag.

## [Unreleased]

### Added

- **Plan complexity (spec v14 → v15, additive; decision 0032):** plans gain an
  optional `complexity: mechanical | standard | deep` — a closed ordinal the spec
  defines by observable task properties (the reasoning depth the work demands of
  its taker, never its size: `mechanical` = the plan determines the change and
  verification is cheap; `standard` = local design judgment inside the declared
  contexts; `deep` = novel or cross-context judgment with a wide blast radius) —
  so the runtime binding can size the dispatched session without a model name,
  effort level, or price ever entering the domain root (0019, 0024). The
  reference `dispatch.yml` maps each tier to `--effort`, `--model`, and
  `--max-budget-usd`, naming both explicitly on every branch including the absent
  one — an omitted `--model` inherits the runner's own configured default, which
  would make the same plan dispatch differently per runner (against the
  reproducibility boundary guarantee) and silently fund unscoped work at whatever
  tier that operator happens to prefer. Absent means the stated default,
  `standard` — the unmarked case, since both other tiers are deliberate
  departures from it. Closed and spec-side rather than an open
  registry like `type:`, because the portable coder must read one meaning in
  every root — the same reason `status` is a spec enum. Meets the
  drives-the-runtime bar 0029 set for `ready` and 0026/0030/0031 applied when
  refusing `parent:`, `progress:`, and `pr:` — this field changes what dispatch
  does, which those did not; depth-only by design, since volume is what plan
  decomposition (0026) answers; readiness untouched, since `deep` is not
  underdetermined — readiness item 9 tests business outcomes, `deep` grades
  construction. Kept honest by two behavioral clauses: the coder escalates a
  mis-scoped tier as a blocker instead of pushing through, and may propose a tier
  on its residue draft for the owner to ratify at release. Surfaces updated: the
  plan schema (`spec/model.md`, `template/conventions.md`), conventions-lint item
  4, the runtime companion (`spec/runtime.md` — "Plan dispatch" and the reference
  binding), `template/.github/workflows/dispatch.yml` (the tier → effort/model/
  budget mapping), the conventions runtime-binding section, `/trellis:plan` steps
  6–7, the coder, the conventions skill's implementation-role procedure, and the
  version pins (`README.md`, `template/decisions/0000-adopt-trellis.md`, and the
  eval skeleton re-pinned — all three instance pins were stale at v13, having
  already missed v14). Eval fixtures themselves are untouched: the field is
  optional and `evals/grade.mjs` reads no plan frontmatter.
- **`trellis:coder` (`agents/coder.md`) and decision 0031:** the implementation
  holder — the producer the dispatch queue (0029) was missing. The steward
  enforces form and focus challenges worth; neither writes authored content, so a
  plan landing in code had no agent to take it. The coder binds to the root,
  reads the mandate of the role it acts as (under dispatch, the plan's `owner:` —
  never a role named `coder`), and does four things: **gates on readiness**
  before any code, flipping an underspecified plan `ready → blocked` and
  escalating the failed items as questions rather than guessing (blocking, not
  leaving it `ready`, is what drains the queue instead of re-burning a session
  every tick — the stall stays visible as item 9 dwell); **runs to completion or
  to a blocker**, claiming with `ready → active`, implementing within the plan's
  `contexts:` under the effective automation class, verifying against the
  context's own tests and contracts, and escalating rather than working around
  anything that needs a human; **delivers code as a PR** — draft while
  unfinished, ready for review when the done criterion is met, never self-merged
  (the class expressed in the binding decides who lands it, and `core` waits for
  its owner); and **files residue as one `draft` plan**, never `ready`, since an
  agent that queues its own next job closes a loop with no human in it. It never
  retires a plan — a merged PR is the completion event and the verdict is the
  owner's. Non-normative: spec stays v14, no schema change, no new status, plane,
  or lint item — an agent is a holder form (Automation shapes).
- `checks/plan-readiness.md`: the canonical definition of *ready* — items 1–8 for
  any taker (objective vs task list, a done criterion the taker can evaluate
  itself, resolving refs, determinable change mechanics, an owner whose mandate
  covers it, authority named not discovered, no deferrals, decisions recorded)
  and 9–11 for an agent taker with no human present (the underdetermination test,
  a reachable and writable target, verification that exists). Shared on the 0011
  precedent: `/trellis:plan`'s release offer checks it and the coder re-checks it
  at pickup, so a plan never leaves `draft` on a bar the taker then fails it on.
  A plan passing 1–8 and failing 9–11 is releasable to a human holder, not an
  agent one.
- Adoption is a conventions-skill procedure, not a template role: the steward and
  focus ship in `template/org/` because they operate the *model*; an
  implementation role operates the *business* — contingent (a bakery has no
  codebase), context-specific in scope, and plural in a domain with several
  code-bearing contexts. Surfaces updated: `spec/runtime.md` (an "Implementation
  holders" note in the reference binding), the conventions skill ("Adopt an
  implementation role"), `/trellis:plan` step 7, `template/conventions.md`'s
  approval-gate bullet, and the README. No eval suite ships with it — `ref.md`
  holders are lint-exempt (item 6) and a coder fixture needs a seeded codebase
  plus a forge, which `evals/` does not model yet.
- Execution health pattern (`spec/patterns.md`) and decision 0030: the one
  metric family portable across every domain, because it reads the machine
  rather than the business — census (plans per status, cut by type, owner, and
  subdomain class), flow (entries, releases, closures, and dwell per status from
  git history), and mix (shares, WIP per owner, cycle time as WIP over closure
  rate). It refines the two misleading readings: percent-complete is rejected —
  no progress field exists and none should be added; dwell and movement (no
  commit within a cadence) are the mechanical substitutes, and decomposition is
  how a domain buys resolution. Closure rate measures clearance, not success,
  since `retired` conflates shipped/abandoned/superseded — split it with an
  anchored verdict section, not a schema change. Structural stance: these are
  instruments, not goals — a generated plan board under `metrics/actuals/`
  (steward-written, threshold-bearing, stale past the sweep cadence per rule 5),
  never a `definitions.md` entry, because a definition earns a target, a target
  earns a plan, and a plan about plan throughput is Goodhart with a mandate or
  spec rule 8 firing. Its use is pre-computing the mechanical fraction of the
  effectiveness walk (stalled ready queue, stalled blockers, plans past their
  horizon, WIP on generic subdomains) into sorted queues; its stated limit is
  that a perfect board says nothing about whether the plans are worth running.
  Non-normative filing only — spec stays v14, no schema change, no new lint
  item; the conventions skill gains a placement row so the numbers don't get
  misfiled into `definitions.md`, and the steward needs no new authority (plan
  boards were already in its generated-view list).
- **Plan readiness & dispatch (spec v13 → v14, additive; decision 0029):** plans
  gain a `ready` status between `draft` and `active` — the owner's deliberate
  release of a specified plan for a taker. A new `plan dispatch` scheduled ritual
  (`org/steward` as operator of record) and a
  `template/.github/workflows/dispatch.yml` cron scan `plans/` for
  `status: ready` and start one `/trellis:act {owner} advance …` per plan; the
  holder branch of `act` routes an agent-owned plan to autonomous execution and a
  human-owned plan to a handoff, and the taker flips `ready → active` on pickup
  (an idempotent queue). Dispatch is `schedule` + `act` composed — no new plane
  or contract service, the action-sibling of the `focus` ritual. Surfaces
  updated: the plan schema (`spec/model.md`, `template/conventions.md`), the
  runtime companion (`spec/runtime.md` — a "Plan dispatch" section + binding
  note), `/trellis:plan` (offer `ready` or `active` on persist), the steward
  mandate and agent, `rituals.md`, and the conventions runtime-binding section.
  Kept honest by conventions-lint item 4 (now enumerates the legal plan statuses)
  and plan-effectiveness item 18 (a `ready` plan the dispatcher never drains is a
  stalled queue), with `ready` counting toward the coverage walk. Existing
  `focus` eval fixtures unaffected.
- `template/AGENTS.md`: a succinct, harness-neutral orientation card at the
  domain root — states that the repo is a Trellis domain, routes an agent to
  where each kind lives, and names the gates on every edit (owner/provenance,
  append-only decisions, act-under-a-role, no secrets). It defers all
  authoritative detail to `conventions.md` rather than duplicating the schemas,
  and carries the standard `provenance: authored` / `owner: <owner>` frontmatter
  so it stays lint-clean (the `<owner>` placeholder is resolved by the existing
  scaffolding step).

- Plan decomposition pattern (spec) and decision 0026: a large effort is an
  umbrella plan plus flat sibling sub-plans (`plans/{parent}-{piece}.md`),
  each with its own lifecycle, grouped by a registered family tag — never a
  `plans/{plan}/` folder, now named as a lint item 11 violation (like
  root-level `skills/`, per 0021). Surfaces updated: the conventions skill
  (placement row + never-create list) and `/trellis:plan` step 2.

- `/trellis:focus` + `trellis:focus` (decision 0025): plan-effectiveness
  evaluation as a command-and-role pair — coverage gaps, metric movement,
  attention allocation against `core-ranking`, blockers/risks, and challenges
  to value-dead plans, per a new shared checklist
  (`checks/plan-effectiveness.md`) with a structured, dedupable finding record.
  The template ships an advisory-only `org/focus/` role (local mandate, ref
  holder to the plugin agent). Escalation-only in v1: no report artifact, no
  candidate kind — accepted candidates graduate through `/trellis:plan`.
- Evals for the effectiveness prompt (`evals/`): a cross-cutting harness — a
  suite-parameterized headless runner (`run.sh`, `--ritual` for the scheduled
  path), a deterministic grader (`grade.mjs`) matching finding records on
  kind/item/refs and asserting the never-writes boundary on every run, a
  universal fixture `skeleton/`, and a shared clean root (`fixtures/healthy`,
  the precision guard) — plus the first suite, `evals/focus/`: five
  purpose-built defect deltas with per-fixture grading contracts and an
  `org/focus` overlay. Defect fixtures stay per-suite by design; machinery,
  skeleton, and clean roots are shared.

### Changed

- **Plan dispatch runs with the interactive tool surface (non-normative; no spec
  change):** `template/.github/workflows/dispatch.yml` drops `--allowedTools` and
  moves from `--permission-mode acceptEdits` to `auto`. The previous allowlist
  (`Read Grep Glob Write Edit Bash(gh *) Bash(git *)`) gave a code-writing holder
  no way to run the build and test commands of the contexts it implements, so it
  could never satisfy the coder's own "verify before you call anything done" rule
  and every PR it opened stayed draft by construction — the workflow's own comment
  had conceded this since the coder landed (0031). What bounds the dispatched
  agent is unchanged and was never the flag list: the role's mandate authority,
  the effective automation class (a `core` change is proposed, never landed), the
  plugin's `PreToolUse` gate on `provenance: generated` and committed decisions
  (which runs ahead of any permission check, so it survives the mode change), and
  branch protection on the PR. `ingress.yml` and `rituals.yml` keep the allowlist:
  ingress acts on externally-filed issues, which `conventions.md` treats as
  untrusted input, and the steward and focus roles never needed a build toolchain.
- **Breaking (spec v12 → v13):** strategies declare economic lineage —
  `funded-by:` edges naming where the value each produces is captured
  (`self`, another strategy, or an external ref), with
  `relation: current | intended` riding the edge (a conversion thesis
  documents, never sustains), per a new spec rule 12, an Economic lineage
  pattern, and decision 0028. A committed strategy with no sustaining edge is
  an economic orphan — same spirit as an orphaned subdomain — and discarding
  a strategy orphans its funding dependents exactly as it orphans induced
  subdomains. Surfaces updated: lint items 19–21 (edge validity, economic
  orphans, capture point), plan-effectiveness items 16–17 (appended
  Economic lineage group — funder health challenges the producer's spend,
  stalled conversion theses; 1–15 stable for the eval contracts), the
  derivation sweep (steward agent + template ritual row), both focus
  surfaces' evidence base, `/trellis:plan` research, rationale premise 1's
  grounds, the template (conventions schema, strategy stub, `economics.md`
  as the narrative over the skeleton, README, v13 pin), and the eval
  fixtures (self-funded; skeleton re-pinned). Instances re-pin per lint 18 —
  one `funded-by:` block per committed strategy.

- **Breaking (spec v11 → v12):** strategy `status:` becomes the six-stage
  maturity ladder — `raw | defined | validated | implemented | established |
  discarded` — per a new Strategy maturity pattern (spec) and decision 0027.
  Each stage names the work owed (refine, validate, implement, monitor,
  harden) and the metrics that read it; the old vocabulary survives as bands
  (committed = `validated|implemented|established`, the inducing band —
  rule 11's phrasing unchanged), and `discarded` is the sole terminal, with
  the successor's `supersedes:` distinguishing superseded from evidence-killed.
  Surfaces updated: plan-effectiveness items 14–15 (appended; 1–13 stable for
  the eval contracts), lint items 13/14/17, both focus surfaces (strategies
  past `raw` enter the walk), `/trellis:plan` step 2 (stage-matched plan
  type), the template (conventions schema, strategy stub, README, v12 pin),
  and the eval fixtures (migrated to `implemented`). Instances re-pin per
  lint 18 using the ADR's migration mapping.

- The template's `plan review` ritual row is replaced by `focus` (executor
  `org/focus`, headless-capable — added to the workflow's RITUALS default);
  `/trellis:ritual` step 3 generalized from the steward to any plugin-agent
  holder. Binding surfaces updated: `spec/runtime.md` session row, the
  template's "Runtime binding" section, and a "Challenge the plans" procedure
  in the conventions skill.

- Moved the founding map out of `problem/` to a root-level `market.md` (decision
  0023): the invariant layer (needs as `## N-{slug}` anchors, jobs-to-be-done,
  market, personas) now lives in its own artifact, and `problem/` holds only
  induced subdomains — one role per directory. Strategies ref `market.md#n-{slug}`.
  Refines 0013's filing; stratification unchanged. Path updates across `model.md`
  (hierarchy, `need:` schema, rule 11), `rationale.md`, `patterns.md`, the
  conventions skill and lint, the steward mandate/agent, and the template.

- Restructured the specification into a content-named document set: renamed
  `spec/trellis.md` → `spec/model.md` (normative core + front door; rule 10 /
  decision 0005), extracted the premises to `spec/rationale.md` (grounding) and the
  non-normative Patterns & practices to `spec/patterns.md` (mirrors
  `spec/runtime.md`); decision 0022. Normative content unchanged; spec stays v11
  (filing-only, per 0018).

### Added

- Automation shapes pattern (spec) and decision 0024: automations classify by
  who triggers them and their relation to judgment — deliberate action →
  `act` invocation, recognized-situation know-how → skill (placed per
  Cross-cutting procedures), bounded identity → role (agent as holder form),
  standing cadence → `rituals.md` row, no-judgment invariant → the gate. The
  README's "Extending the plugin" now states the member-type selection rule
  (verb → command, topic → skill, role → agent, guard → hook), with decision
  0020 as the worked example.

- Cross-cutting procedures pattern (spec) and decision 0021: skills have
  exactly two homes — `solution/{bc}/skills/` for business procedures,
  `org/{role}/holder/skills/` for identity-bound agent technique — and a
  root-level `skills/` is a grouping-not-a-kind violation. A "context-free"
  procedure resolves to a convention, a ritual, a runtime concern, or a
  missing bounded context. Lint item 11 now names the root `skills/` case;
  the conventions skill's placement guide and "Never create" list updated.

- `/trellis:plan` (`commands/plan.md`, decision 0020): plan authoring that
  rides the harness's plan mode — read-only research, `ExitPlanMode` approval,
  then a conventions-compliant artifact persisted to `plans/{slug}.md`
  (registered `type:`, resolving refs, `status: draft` with an activation
  offer); registry additions land post-approval; self-lints against checklist
  items 1/4/7/8. Binding surfaces updated: `spec/runtime.md` session row, the
  template's "Runtime binding" section, and a "Create a plan" procedure in the
  conventions skill.

- Runtime as contract-plus-bindings (`spec/runtime.md`, decision 0019): three
  trigger planes reduced to one `act(role, input)` primitive; `/trellis:act`
  and `/trellis:ritual` commands; a deterministic enforcement gate (`hooks/`:
  append-only decisions, no hand-edits to `generated` artifacts without an
  acting-role marker, frontmatter warnings on new artifacts); template
  reference workflows for the scheduled (cron rituals) and event-driven
  (`role:{name}` issue labels) planes, with forge issues/PRs as the escalation
  and approval channels; a "Runtime binding" section in the template's
  `conventions.md`.

- Stratified model (spec v11): `strategy/` as a first-class kind
  (`status: aspirational|committed|retired` — only committed induces), the
  founding map (`problem/README.md`: needs as `## N-{slug}` anchors,
  technology-free), `induced-by:` provenance edges on subdomains with
  classification per edge, spec rule 11, and the Pivot / Stratified alternation /
  Classification inversion patterns. Decision records 0013–0018.
- Lint rules 13–17 (strategy validity, orphan detection, core-ranking,
  technology-free founding map, incomplete pivot) and a rewritten rule 3
  (derivation edges).
- Derivation sweep: a third steward ritual escalating downstream artifacts when
  the founding map or a strategy changes, and flagging orphaned subdomains.
- Template: `strategy/first-strategy.md` stub; scaffold order now founding map →
  first strategy → induced subdomains.

### Changed

- Subdomain classification moved from the node (`class:`) to the
  (subdomain, strategy) edge; effective automation policy is the strictest class
  across committed edges, orphans defaulting to core. No migration path — pre-1.0,
  zero usage-hours.

- Initial Trellis plugin prototype — the specification (`spec/trellis.md`), the
  domain scaffold (`template/`), the `trellis:conventions` skill, the
  `trellis:steward` agent, the shared lint checklist (`checks/conventions-lint.md`),
  and decision records 0001–0011.
- Self-distributing marketplace catalog (`.claude-plugin/marketplace.json`): the
  `trellis` plugin under the `scarmuega` marketplace, installable with
  `/plugin marketplace add scarmuega/trellis` then `/plugin install trellis@scarmuega`.
- Versioning policy (`decisions/0012`) and this changelog.
