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
