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

- **The canonical local runtime: `trellis serve` (non-normative; decision
  0038):** Stage 3 of `spec/runtime.md`'s staging plan — landed, and not as
  staged. That entry promised a thin ingress-only dispatcher on the Claude
  Agent SDK, pulled by an event the forge could not relay; that pull never
  fired. What fired was that the reference binding made the *forge* structural
  for three of seven contract services, so a domain operated from a laptop
  against a bare git remote had an interactive plane and no clock at all;
  0036's chartered escalation push seam had no substrate to attach to; and 0037
  collapsed the daemon's cost from a build to a loop over the existing kernel.
  `trellis serve` is a daemon mode of the same binary that **owns the clock**
  (rituals fired on their `rituals.md` cadences, read at every pass so nothing
  needs keeping in step with a cron; the dispatch scan run in-process with hold
  semantics and the `ready → active` idempotence unchanged), **invokes
  harnesses through argv command templates** in a committed `runtime.toml` so
  `claude -p` is the reference adapter rather than a hardcode and a placeholder
  fills exactly one argument slot, **serves the tree read-only** over a JSON API
  that is a projection of what the commands already print — `plan list`,
  `show`, `escalate list`, `dispatch scan`, the rendered views — plus an
  embedded static plan board over that API with no build step and no package
  manager, and **defines the escalation channel-adapter seam** with no adapter
  shipped, so the pull surface is the live channel and a configured-but-absent
  transport is refused at startup rather than silently dropped. Read-only is
  structural, not a setting: there is no write path, which is also why the
  surface is not ingress. Serve is never the interactive plane and never
  judgment — it starts sessions, roles act. The forge binding stays the
  *reference* (it alone covers all seven services, serve defers ingress); serve
  is the canonical *local* binding, and `template/` now ships both with
  `conventions.md` making the instance pick one — two clocks run every ritual
  twice. New limits recorded rather than left to be discovered: a sleeping
  machine misses cadences (catch-up fires once, never once per missed window),
  the board and session logs are single-machine, and the API is unauthenticated
  behind a loopback default. Spec stays v16; no lockstep entry, argued rather
  than omitted — serve encodes no normative rule.

- **The deterministic kernel: a `trellis` CLI (non-normative; decision 0037):**
  Stage 2 of `spec/runtime.md`'s staging plan, pulled by dogfooding — the
  conventions the members walk as prose had stopped moving, so twenty of the
  lint checklist's twenty-four items were being re-derived (and occasionally
  fumbled) by an LLM every ritual, the dispatch scan lived as awk one-liners,
  the gate as a regex-over-4KB Node script, and every generated view the spec
  names had gone unimplemented because prose is a poor place to keep a
  generator. `cli/` is now a Rust crate shipping one binary: **lint** (the
  mechanical items, findings shaped path + item + message + owner for relay to
  each owner, and a `judgment` array naming exactly what remains — 7, 16, 2's
  unstamped artifacts, 23's backing read — so the split is explicit rather than
  silent), **`dispatch scan`** (the awk ported whole, hold semantics and
  fail-closed dangling targets included, with the complexity→session mapping
  still tuned in `dispatch.yml` via `--map` so no provider name or price enters
  the binary — 0032 intact), **`gate`** (the PreToolUse hook, `gate.mjs` ported
  guard-for-guard, fail-open through a panic), **lifecycle porcelain**
  (`plan release|claim|block|unblock|retire` refusing illegal source states,
  `block` writing the open escalation record lint item 24 demands in the same
  move, format-preserving frontmatter edits that verify by re-parse and refuse
  rather than reflow), **five view generators** (board, CODEOWNERS, tags,
  orgchart, escalations — pure functions of tree plus git, so same tree and day
  render byte-identical), **readiness**'s mechanical share tiered honestly as
  pass/partial/judgment, plus `scaffold`, `rituals cron --check`, and the
  derived-value queries every agent used to recompute per session. Two keys
  added, both binding-owned rather than model-owned: `generated-by:` on
  generated views (which turns lint item 2 into a byte-diff for them) and an
  optional `github:` on `holder/ref.md` for CODEOWNERS handles. **The kernel
  changes in lockstep with the model** — a model change is not complete until
  the kernel carries it *in the same commit*, since a lagging kernel reports a
  clean root against conventions the root no longer has, with the authority of
  a deterministic check. Prose could not hold that line (the v14 bump shipped
  with all three instance pins stale at v13), so `cli/tests/lockstep.rs`
  enforces it: rule table against `checks/conventions-lint.md`, readiness walk
  against `checks/plan-readiness.md`, embedded spec version against
  `spec/model.md` and every shipped pin, closed enums against the schema prose —
  with CI (this repo's first) triggering on `spec/**` and `checks/**`. Surfaces
  updated: `hooks/hooks.json` (prefers `trellis gate`, falls back to `gate.mjs`
  until distribution lands, so the guarantee never lapses),
  `template/.github/workflows/dispatch.yml` (the awk loop replaced by the scan +
  jq), `agents/steward.md` (lint ritual runs the binary and judges the
  remainder; views prefer the generators), `spec/runtime.md` (gate binding row,
  Stage 2 marked landed with permission-profile compilation still open),
  `template/conventions.md` (dispatch + CODEOWNERS binding rows), and the README
  (layout, quick start, a "Changing the model" procedure). No schema, no Rule,
  no new status or kind, spec stays v16; eval fixtures untouched — the kernel
  reads them as test inputs and writes nothing back.

### Removed

- **The GitHub Actions binding (non-normative; decision 0039):** the three
  template workflows (`dispatch.yml`, `rituals.yml`, `ingress.yml`) and the
  `trellis rituals cron` subcommand with its `cli/src/rituals.rs` module are
  gone. 0038 had shipped both clocks and made each instance pick one; on the
  evidence — the workflows were never really used — that was the wrong call.
  An unexercised alternative is not free: it is described in the spec, the
  template, and every instance's own `conventions.md`, chosen against in every
  scaffold, and stale before anyone notices. `trellis serve` is no longer "the
  local binding" beside a forge one; it is **the** binding, and `spec/runtime.md`
  now describes one. **The forge keeps review and loses triggering**: PRs remain
  the approval gate, CODEOWNERS is still generated, branch protection still makes
  core-class review structural, and `github:` stays on `holder/ref.md` — none of
  that is Actions, all of it is `spec/model.md`'s automation policy reaching its
  mechanics. What goes is the forge as a trigger. **`ingress` is now unbound and
  says so**: it was bound only by `ingress.yml`, so the event-driven plane has no
  binding at all, recorded as a binding choice in `spec/runtime.md` and the
  template rather than left to be discovered — an outside event reaches the
  domain when a human brings it into a session. Binding it from the daemon's
  already-open socket is refused deliberately: that would be a trigger plane with
  no mandate behind it, which is the argument 0038 made for the serving surface
  being read-only by construction. Cron generation retires with the clock it
  served — the daemon reads `rituals.md` at every pass, so there is no cron to
  emit and no drift to check. The costs are recorded rather than softened: the
  clock now runs where the daemon runs, so a sleeping machine misses cadences
  (catch-up fires each overdue row once); there is no event-driven plane at all;
  and the serving surface is single-machine. Spec stays v16 — the three trigger
  planes remain the shape a runtime must have, and this binding leaves one empty,
  which the conforming-binding checklist now asks bindings to declare.

### Changed

- **Escalations are in-repo records, not forge issues (non-normative; decision
  0036):** the escalation channel was a forge issue, hardcoded as `gh issue
  create` in four members, which split one event across two systems — the
  coder's `ready → blocked` flip landed as an artifact edit while the question
  that would clear it went to an issue tracker no later session reads. That is
  the Loop observability failure applied to escalations: a loop whose
  termination condition is filed outside the tree. An escalation is now body
  content in the artifact it concerns — a fenced `yaml` block under
  `## Escalations` carrying `raised`, `by`, `to`, `status: open|resolved`, and
  the question asked of a human — so a blocked plan states its own blocker and
  the trail is git history. Not a new kind: an escalation has no lifecycle or
  owner of its own (rule 7), and an artifact carries zero or many, so it is
  neither a directory nor frontmatter. **Only the artifact's `owner:` writes a
  record**, escalations being authored content like any other: an agent acting
  *as* the owner writes its own (the coder owns the plan it was dispatched on),
  while a role whose mandate does not reach the artifact reports the finding for
  the owner to transcribe — which leaves `org/focus` ("writes nothing into the
  root") and `org/steward` ("never edit `authored` artifacts") exactly as strict
  as they were, and resolves the standing contradiction where the metric sweep
  told the steward to "annotate plans" its boundary forbade. New lint item 24
  makes the channel checkable as the forge never was: records well-formed, every
  `blocked` plan carrying an open one, none in a `generated` artifact — a held
  plan (`awaits:`) explicitly exempt, carrying no defect. The forge keeps
  ingress and PR approvals; only escalation moves. Two costs taken knowingly and
  recorded as binding limits: a committed record notifies nobody (the
  contract service's audit-trail half is satisfied, its
  channel-they-already-watch half is not), and an advisory role's finding is
  durable only once its owner transcribes it, so a headless ritual nobody reads
  leaves findings in a workflow log — the pull-triggered fix is a per-role
  escalation inbox, not a carve-out in the ownership rule. Surfaces updated:
  `spec/runtime.md` (binding row, an escalations paragraph, two known limits),
  `spec/patterns.md` (Loop observability), `template/conventions.md` (an
  "Escalation records" schema + the runtime-binding row), `template/rituals.md`,
  `agents/coder.md`, `agents/focus.md`, `agents/steward.md`, `commands/act.md`,
  `commands/focus.md`, `commands/ritual.md`, `checks/conventions-lint.md` item
  24, `checks/plan-readiness.md`, and the conventions skill (map, placement
  guide, a "Raise an escalation" procedure, lint summary). No schema, no new
  status, no new kind, spec stays v16; eval fixtures untouched.

### Added

- **Contract steering (non-normative; decision 0035):** contracts were the only
  load-bearing kind whose loop was never closed — one consumption instruction
  (the coder reads and satisfies them) and zero production triggers anywhere,
  which the dogfooding evidence surfaced as ~50 plans completed with no
  contract involved. The produce side is now wired at every point where work
  touches a seam: readiness item 11 gains the interface clause (an
  interface-shaped done criterion is verified by its contract, existing or
  hatched by the plan — tests on one side of a seam do not verify the seam),
  checked twice as ever at release and pickup; the coder gains an "interfaces
  land as contracts" implement rule and "an interface left in prose" as
  residue; `/trellis:plan` reads the touched contexts' `contracts/` at
  research and asks the interface question at drafting. Two standing pulls
  keep it honest: lint item 23, checking each `context-map.md` relation's
  claim against its artifact (`published-language`: the publishing context's
  `contracts/` holds it; `conformist`/`acl`: a pinned ref names the upstream
  language conformed to or translated — conforming to something unnamed is the
  same failure inverted; `customer-supplier` claims negotiation, not an
  artifact — its interface may run larval until the seam steering hatches it),
  and the derivation sweep now watches `contracts/` edits,
  escalating every context the map relates to the publisher — the context map
  is the consumer registry, and this is the payoff that makes producing a
  contract worth it. Trigger discipline throughout: every clause fires on
  seams, never on plans generally — blanket contract ceremony is the
  accumulation problem the Requirements pattern warns against. Deferred by
  pull: an effectiveness item for seam-crossing plans without a contract, and
  a `contracts:` frontmatter ref list (no consumer branches on it). Surfaces
  updated: `checks/plan-readiness.md` item 11, `agents/coder.md`,
  `commands/plan.md` steps 5–6, `checks/conventions-lint.md` item 23,
  `agents/steward.md` (derivation sweep), `template/rituals.md`, and the
  conventions skill (lint summary + a "Hatch a contract" procedure). No
  schema, no Rule, spec stays v16; eval fixtures untouched —
  `published-language` appears in no fixture and readiness items are not
  graded.
- **Plan sequencing (spec v15 → v16, additive; decision 0033):** plans gain an
  optional `awaits: [plans/{other}.md]` — declared sequencing between plans. The
  dispatch scan holds a `ready` plan whose targets are not all `retired`:
  skipped, still `ready`, no status change, retried every tick, self-clearing
  when the last target retires. A hold is never a flavor of `blocked` (defect,
  escalation, human clears it, leaves the queue) — it is an order the owner
  declared, and the plan stays in the queue so no re-release is needed when the
  wait ends. Satisfied means every target `retired`, the only owner-asserted end
  of a plan's lifecycle (there is no `done` — 0030; under dispatch a finished
  plan sits `active` because the verdict is the owner's — 0031), so releasing a
  dependent is an owner-gated governance act, never an inference from forge
  state; the two honesty gaps are instrumented, not accepted — verdict latency
  starving a dependent is plan-effectiveness item 20, a target retired without
  shipping is item 21 (the mirror of rule 11's re-parenting). Named `awaits`,
  not `blocked-by`, so one grep never returns two mechanisms with opposite
  clearing semantics (rule 10). Clears the bar 0026 set when refusing `parent:`
  — dispatch branches on the edge — and gives decomposed families (0026) their
  missing piece: release the whole family at once, drain in dependency order,
  ordering machine-visible instead of umbrella prose. Fail closed on a dangling
  target (warn and hold; lint owns the repair), acyclic by lint (new item 22 —
  a deadlock by construction, in the closed-funding-loop idiom). The coder
  may propose `awaits:` on its residue draft — most often awaiting the plan just
  advanced — and never edits a released plan's edges (new hard boundary).
  Surfaces updated: the plan schema (`spec/model.md`, `template/conventions.md`),
  conventions-lint item 4 + new item 22, plan-effectiveness (walk-order
  preamble, item 18 carve-out, new items 20–21, v16 gate), the runtime
  companion (`spec/runtime.md` — "Plan dispatch" and the reference binding),
  `template/.github/workflows/dispatch.yml` (list-valued frontmatter read + one
  `status:` lookup per target, still zero judgment), `template/rituals.md`'s
  dispatch row, the conventions runtime-binding section, `/trellis:plan` steps
  2/6/7, the coder (pre-gate hold check, residue proposal, boundary), both
  focus surfaces' evidence base, the steward (dispatch skip + derivation-sweep
  flag on plans retired), the conventions skill (map, placement rows, lint
  keywords, implementation-role procedure), the Plan decomposition pattern, and
  the version pins (`README.md`, `template/decisions/0000-adopt-trellis.md`,
  and the eval skeleton — all re-pinned v15 → v16). Eval fixtures themselves
  are untouched: the field is optional, no fixture declares it, and
  `evals/grade.mjs` reads no plan frontmatter.
- **Loop observability (non-normative; decision 0034):** the principle four
  decisions kept re-deriving — every standing loop's exit and progress
  conditions live in artifacts and git, never in agent memory or self-report —
  stated once as a `spec/patterns.md` entry (beside Execution health; grounded
  in rationale premises 9 and 5; ending on the test: erase every participant's
  memory between ticks — does the loop still stop, hold, and resume from the
  tree alone?), plus its uncovered failure mode instrumented as
  plan-effectiveness item 19: a `ready` plan dispatch has acted on across more
  than one cadence with no `ready → active` (or `→ blocked`) flip on the
  default branch — takers dying before the claim, budget burning every tick
  while the queue believes the plan untaken; a flip stranded on a proposal
  branch is the same finding. Complements item 18 (the act that never fires)
  with the act that fires and dies unclaimed, and is deliberately not gated to
  v16 — the failure exists on any dispatching root from v14 on. No schema, no
  Rule, no spec content beyond 0033's bump.
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
