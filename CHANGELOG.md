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

### Changed

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
