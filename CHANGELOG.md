# Changelog

All notable changes to the Trellis plugin are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

**Versioning.** The spec and the release move in lockstep: the version is
`0.<spec>.<patch>`, so spec v21 ships as `0.21.0` and the next spec bump is
`0.22.0` (see `decisions/0056-spec-and-cli-bump-in-lockstep.md`). One number
covers the plugin (`.claude-plugin/plugin.json`) and the kernel
(`cli/Cargo.toml`); the patch component carries plugin- or kernel-only releases
between spec bumps. Every bump is a release: it closes `## [Unreleased]` into a
`## [x.y.z]` section here and gets a matching `vx.y.z` git tag.
`cli/tests/lockstep.rs` fails the build on a half-done one.

## [Unreleased]

### Changed

- **`awaits:` arrows point at the dependency:** both graph renderings — the
  board's `/plans` graph view and `trellis view plan-graph` — drew the edge
  backwards, prerequisite → dependent, on the rationale that the figure should
  read forward in execution order ("what clears when"). Dogfooding says the
  reader expects the opposite: `awaits:` is a declaration of dependency, so the
  arrow should carry that statement and following one should lead to what must
  finish first. Edges now run dependent → target on both surfaces, and the
  board's ELK layout flips with them — dependents on the left, the plans they
  await on the right — so arrows still run left to right in the reading
  direction. Nothing about hold semantics changes; only which end of the edge
  the arrowhead sits on. **Generated `metrics/actuals/plan-graph.md` files go
  stale**: any root that has written one fails `trellis view plan-graph
  --check` (lint item 2) until it is regenerated with `--write`.

## [0.21.0] - 2026-08-15

### Changed

- **Spec and CLI bump in lockstep (decision 0056):** the release version is now a
  function of the spec version — `0.<spec>.<patch>` — instead of a number nobody
  read. `cli/Cargo.toml` said `0.2.0` against spec v21, bumped by hand once and
  made decorative by decision 0037 ("the binary floats on SHA"), so `trellis
  version` printed two unrelated numbers, one of which meant nothing. The spec
  integer was already lockstepped four ways (`build.rs` scrapes `spec/model.md`'s
  title; `cli/tests/lockstep.rs` matches it against `template/trellis.toml`,
  `evals/skeleton/trellis.toml`, and the README); the release version now joins
  them. `.claude-plugin/plugin.json` gains a `version` and stops floating on the
  commit SHA — 0012 deferred that switch until external consumers existed, and
  the roots pinning `spec = N` are those consumers — while
  `.claude-plugin/marketplace.json` deliberately declares none, since
  `plugin.json` wins silently when both are set. A new lockstep assertion,
  `release_version_tracks_the_spec_version`, checks all four halves: major is 0
  and minor is the spec version, the plugin manifest matches the crate, the
  marketplace entry carries no version, and `CHANGELOG.md` holds a section for
  the current version — so a spec bump cannot land without its release. **The
  cost 0012 named is now accepted**: installers no longer follow `main` HEAD, and
  a forgotten bump freezes every install; CI catches a half-done bump, not a
  forgotten one, and the patch component is the valve for reaching installers
  without a spec bump. Updated: `cli/Cargo.toml`, `cli/Cargo.lock`,
  `.claude-plugin/plugin.json`, `cli/tests/lockstep.rs`, `README.md`, and this
  file.

- **Machine config splits out of conventions.md (spec v20 → v21, Breaking;
  decision 0054):** the per-instance `conventions.md` — a hand-copied fork of
  the spec's normative text — is gone. Dogfooding across eight roots showed
  the copies drift apart and rot: one root's copy taught a plan lifecycle
  several spec versions stale, with the full authority every session grants
  the instance's own file. What the file actually held splits three ways.
  Machine-read config — the required `spec` pin, the plan-type / tag /
  decision registries, the `carried` prefixes, and the `archive.after_days`
  retention horizon — moves to a new **`trellis.toml`**, which replaces
  `conventions.md` as the root marker: schema-validated at load
  (`cli/src/domain.rs`, unknown keys refused, mirroring `runtime.toml`), so
  the `archive after N days` prose regex and the heading-slug registry
  scraper (`markdown::registry_items`) are deleted. A file that does not
  parse refuses the whole load with a hard error naming it — never a lint
  pass over empty registries — while root *discovery* stays existence-only,
  so a corrupt file cannot brick the write gate. Instance prose — boundary
  guarantees, secrets policy, runtime-binding choices — stays markdown in an
  optional, much smaller **`domain.md`** (`Kind::Conventions` →
  `Kind::Domain`), whose runtime-binding line keeps `decisions/0000`
  reachable for item 26. The restated spec text is deleted: the spec is
  authoritative — the escalation-record schema's home moves to
  `spec/runtime.md`, the frontmatter schemas' to `spec/model.md` — and lint
  item 18 reads the pin from `trellis.toml` (owning exactly the mismatch
  case; a root that pins nothing fails to load at all). No lint item added,
  removed, or renumbered. **Migration is a hard cutover**: an old-style root
  fails discovery with "markers: trellis.toml, problem/, solution/, org/" —
  extract its registries and horizon into `trellis.toml`, `git mv
  conventions.md domain.md`, delete the restated sections, and re-pin after
  reviewing the intervening spec changes; there is deliberately no `trellis
  migrate` command. The `PROCEDURE_BRIDGE` sentence in the default prompts
  changed with the template's copy — the two are kept identical by hand.
  Updated: `spec/model.md`, `spec/runtime.md`, `spec/patterns.md`,
  `checks/conventions-lint.md`, `template/` (new `trellis.toml`,
  `domain.md`, README, AGENTS, rituals, steward mandate, `.gitignore`,
  `runtime.toml`, `decisions/0000`), `hooks/gate.mjs`, `commands/act.md`,
  `commands/plan.md`, `commands/refine.md`, `commands/focus.md`,
  `commands/ritual.md`, `agents/steward.md`, `agents/focus.md`,
  `skills/conventions/SKILL.md`, `evals/skeleton/`, `evals/README.md`,
  `README.md`, and the spec pins.

### Fixed

- **One dispatcher per root, one claim per plan (decision 0053):** dogfooding
  found two `trellis dispatch run` daemons live on the same root for seven
  hours; both dispatched the same plan, and the two sessions spent 28 minutes
  editing the same files in the same uncommitted checkout. Two independent
  faults, both now closed. The `dispatch.pid` lock was advisory — read the
  pid, ask `kill -0`, write your own — which is not atomic, whose `Drop`
  removed the file unconditionally (so any exiting dispatcher deleted a live
  one's lock), which `--once` skipped entirely, and which is blind by
  construction to a daemon started from a binary that predates it. It is now
  an exclusive `flock` held for the process's lifetime, rechecked against the
  path, released by the kernel however the holder dies — no stale lockfile to
  remove by hand — with `Drop` removing only its own file and `--once` taking
  the same lock. `serve` reads the recorded pid for liveness rather than
  treating the file's existence as proof. And the dispatch guard moved: the
  plan is flipped `ready → active` **before** its session spawns rather than
  on the session's first turn, closing the minutes-long window in which a
  plan already sent to a session still read `ready` to every other scan. A
  failed spawn puts the claim back. Only `act` sessions, only from `ready` —
  the same pair `relinquish` tests on the other end. Two dispatchers can no
  longer both write `dispatch-state.json`, which also closes the
  last-writer-wins clobber that silently dropped a `seen_escalations` entry.
  Updated: `spec/runtime.md`, the default `act` prompt (the session is told
  the plan is already claimed, not to claim it). Not addressed, and named in
  the decision: `trellis rituals` still takes no lock, and worktree isolation
  for dispatched sessions remains its own open question.

- **A plan can no longer strand in `active` (spec v19 → v20, additive;
  decision 0052):** dogfooding found a plan sitting `active` for a day with no
  session running — dispatch enumerates `ready` only, so nothing retried it,
  while the board still showed it in flight. Two faults, both now closed.
  Under `backend = "herdr"` the pool read a pane as settled while the harness
  footer still reported armed background monitors (`2 shells, 1 monitor still
  running`) — a monitor is a standing promise to re-invoke the session, so
  that settle is now a `Waiting` phase: it holds its slot, shows as
  `[waiting]`, is never retired there, and is detached rather than waited out
  by `--once`. Background shells are deliberately not counted (a dev server
  outliving its turn would hold a slot forever). And a session dispatched to
  advance a plan now owes a verdict: ending with the plan still `active` and
  no declared `handoff:` returns it to `ready`, which is what the state
  already is — released, and nobody has taken it. Only `act` sessions, so a
  refine never demotes a plan a human is driving; the retry cooldown bounds
  the return. The rule lives in the kernel (`plan_ops::relinquish`), not the
  daemon. Updated: `spec/model.md`, `spec/runtime.md`,
  `template/conventions.md`, the default `act` prompt, and the three spec
  pins.

### Removed

- **`trellis:coder` (`agents/coder.md`), superseding decision 0031 (decision
  0047):** the plugin ships no implementation agent; each domain authors its
  implementation holder ad hoc as a domain-local package (`holder/system.md` +
  at least one eval, lint item 6). Dogfooding drove it: the first production
  adoption needed a mandate-wins override from day one and was then replaced by
  an inlined package — the agent's generic posture carried domain policy (org
  boundaries, writable surfaces, verification, how much of the readiness bar a
  taker re-checks) only as prose each domain overrode. The steward and focus
  stay portable because they operate the model; implementation operates the
  business. The coder's behavioral clauses survive as documented discipline in
  `spec/runtime.md` ("Implementation holders") and the conventions skill's
  "Adopt an implementation role" procedure; `checks/plan-readiness.md` remains
  the canonical bar, with the pickup-side share now the domain's call. Updated:
  `README.md`, `spec/runtime.md`, `template/conventions.md`,
  `skills/conventions/SKILL.md`, `commands/act.md`, `checks/plan-readiness.md`.

### Added

- **Skills ride the spawn prompt as an index (decision 0055):** a new
  `{skills}` placeholder closes the residue 0050 left ("the skills still
  assume the plugin where humans work"). At spawn the runtime resolves the
  session's skills deterministically — a plan session gets its owner's
  `org/{role}/holder/skills/*` then each `contexts:` home's
  `solution/{bc}/skills/*`, a ritual its executor's holder skills — and
  renders an index: one line per skill, name and path always, description
  when derivable (frontmatter `description:` first line, else first H1, else
  first prose line; primary doc `README.md` → `SKILL.md` → sole `.md`).
  Bodies are never inlined — the session reads a listed file through its own
  tools, so any harness that takes a prompt gets the whole mechanism, no
  plugin dialect. The value is self-contained and empty-safe (heading and
  standing instruction inside; empty string when no skill exists), so the
  default `act` and `ritual` templates name it unconditionally; `refine`
  deliberately does not — refinement is never handed execution procedures —
  though the variable is set for every errand, so a custom template may.
  Compatibility: an old binary refuses a new template naming `{skills}` at
  startup (`tmpl::check`), the designed direction of failure; a new binary
  with old templates renders exactly as before. In passing,
  `template/runtime.toml`'s commented prompt copies are re-synced to the
  embedded defaults they had drifted from (the stale claim instruction and
  the missing verdict paragraph from 0052/0053). No spec bump — no artifact
  schema changes. Updated: `spec/runtime.md`, `spec/patterns.md`,
  `template/runtime.toml`, `commands/act.md`.

- **`handoff:` on plans (spec v19 → v20, additive; decision 0052):** an
  optional declared field naming what an active plan is parked on — a PR
  awaiting its owner's verdict, or any ref a human must move. While set, the
  plan stays `active` and out of dispatch; claiming clears it, since a fresh
  claim is a fresh attempt. Written by the taker with `trellis plan handoff
  <plan> <ref>` (or `--clear`), read by the runtime as one declared field —
  never inferred from a forge, on the same argument decision 0033 made for
  `awaits:`: a merged PR is the taker's event, the verdict is the owner's.

- **Every errand is a template; the only command is act (decision 0051):**
  `[prompts]` in `runtime.toml` becomes a name → template map, and the binary
  embeds `commands/act.md` alone — `{procedure}` always renders the act
  body, and what refine or ritual add on top now lives in their default
  templates (framework-authored, instance-overridable, the `rituals.md`-row
  precedent). Any extra `[prompts]` key is a new operator-requestable errand
  with no recompile: `trellis dispatch request <errand> <plan>
  "<instruction>"`, `POST /api/plans/{slug}/errands/{name}` (with `/refine`
  as the alias the route shipped under), `GET /api/errands` feeding the
  board's Act menu. `act` and `ritual` stay the triggers' own, never
  requestable. Old flat `[prompts]` overrides keep working unchanged.

- **The spawn prompt is a rendering of the command (decision 0050):**
  daemon-spawned sessions no longer open with a slash command only an
  installed plugin can resolve. The binary embeds `commands/act.md`,
  `refine.md`, and `ritual.md`; the default prompts compose a per-session
  header, the computed facts of 0045, and `{procedure}` — the canonical
  command body, frontmatter stripped, refine/ritual rendered together with
  the act procedure they delegate to. The contract stays in the command
  files (0048 upheld); the prompt is a rendering of them, identical at the
  binary's build SHA. `--plugin-root` / `CLAUDE_PLUGIN_ROOT` swap a live
  checkout's `commands/`; an instance that prefers the plugin spelling sets
  `[prompts]` back to `/trellis:act …` and renders exactly as before. The
  interactive plane, the gate hook, and skills still ride the installed
  plugin. New placeholders `{procedure}` and `{executor}`; the herdr
  prompt-delivery needle now cuts at the prompt's first line, which the
  defaults make per-session-unique. Updated: `spec/runtime.md`,
  `template/runtime.toml`, `template/conventions.md`, `template/AGENTS.md`,
  CI trigger paths (`commands/**` now changes the binary).

- **The runtime splits by what its time means (decision 0046):** `trellis
  serve` was one process owning three unlike things, and the seam showed the
  day 0045 landed — an `awaits:` chain released its next plan minutes after
  the daily dispatch pass and the plan sat dispatchable for a day, because
  dispatch's cadence was never semantic, just the fossil of the forge cron
  the scan was ported from. Now: **`trellis dispatch run`** is the pull loop
  — every tick it polls the tree and spawns an act for everything
  dispatchable, no calendar gate; the backstop the cadence provided by
  accident it provides on purpose (`retry_cooldown_secs`, default 900,
  skipping a plan whose session ended without claiming); holds and handoffs
  are announced on transition, never once per tick; and it hosts the
  sessions' MCP back-channel on its own loopback socket, because the channel
  belongs to the process that owns the session lifecycle.
  **`trellis rituals`** is one idempotent pass of the wall-clock work — fire
  what the cadences owe today, drain, record, exit — safe under cron or
  launchd at any frequency, `catchup` keeping its meaning; a ritual's gap is
  part of its meaning (the metric sweep's cadence *is* the freshness window)
  and stays scheduled. **`trellis serve`** is the read-only window: tree,
  views, board, and the dispatcher's `status.json` overlaid on `/api/status`
  with an honest liveness flag — no clock, no spawning, down without
  consequence. State splits along the keyspace
  (`dispatch-state.json` / `rituals-state.json`, legacy `state.json` read
  once), herdr adoption filters by key prefix so the two spawners never
  fight over a session, the acting-role marker gains a lockfile for its two
  writers, and `trellis inbox` finds the dispatcher first. Gone: `serve
  --once/--no-http/--tick-secs/--map/--dry-run` (their homes are `dispatch
  run` and `rituals`) and `scheduler.dispatch_cadence` (parsed, ignored,
  noted obsolete).

- **The act ceremony's deterministic share runs in the runtime (spec v19;
  decision 0045):** a measured dispatched act spent ~10–15% of its tokens and
  ~25 turns on ceremony no judgment touches — and whole sessions on plans
  that deterministically shouldn't have spawned one. Four moves, one
  decision. The dispatch scan now runs the mechanical readiness share
  (`readiness.rs`, which always had the verdict and never had a caller in
  the daemon) and **holds** a failing `ready` plan the way it holds
  `awaits:` — nothing written, failed items named on the log, board, and
  `/api/dispatch`; the block-flip and the escalation record stay a mandated
  session's, which is why a hold and not an auto-block. `holder/ref.md`
  gains an optional `kind: agent | human`: a declared human's plans become
  reported **handoffs** instead of sessions that read everything and stop at
  never-impersonate; undeclared routes exactly as before, and lint warns —
  never fails — when a role owning ready plans leaves it unsaid. The daemon
  owns `.trellis/acting-role` for the sessions it spawns (one line per live
  session, stamped at spawn, removed at retirement, reconciled at startup),
  which also deletes the dotfile permission dialog that blocked unattended
  acts. And the act prompt now carries computed facts — a new
  `{escalate_to}` placeholder, "the precheck already ran" — and names the
  CLI verbs (`trellis plan claim`, `trellis plan block --asks`,
  `trellis escalate add`) that replace hand-edited frontmatter and
  hand-written YAML. On the plugin side, act's holder branch adopts plugin
  agents **inline** instead of Task-delegating, removing the second context
  that re-read ~40KB of mandate, conventions, and plan. The cost is stated
  where it lives: the deferral-token scan reads plan bodies, wider than
  v18's declared-fields-only phrasing, and a readiness hold is quiet by
  design — it re-enters the scan each tick and toasts nothing.

### Changed

- **The coder agent commands instead of explains (follows 0045):** the scan
  now computes the plan's change mechanics — a new `{automation}` placeholder
  carries each subdomain's effective class and the strictest one into the act
  prompt (`DispatchItem.automation`, visible on `/api/dispatch`; empty for
  rituals) — so `agents/coder.md` stops teaching the derivation and stops
  loading the conventions skill for it. The rewrite (164 → 125 lines) reads
  only the mandate, the plan, and three named `conventions.md` sections
  (boundary guarantees, runtime binding, secrets policy); the flip/record
  mechanics the CLI enforces are commanded in one line each rather than
  re-explained; the boundaries keep only what judgment can violate, with a
  note that the gate holds the mechanical ones. The judgment-dense sections —
  contracts as published language, verify-before-done, PR trail, domain state
  never in the proposal, residue discipline — are kept whole: those are
  behaviors, not derivable facts.

- **The steward and the conventions skill stop re-deriving what the CLI
  owns:** the steward's plan-dispatch section collapses to "run
  `trellis dispatch scan`, act each row" (the daemon case never invokes it at
  all); its bind step reads the two `conventions.md` registries it needs
  instead of loading the whole skill; the metric sweep points at
  `trellis lint --items 12` for freshness and the derivation sweep at
  `--items 3,14,17,19,20,21` for the standing classifications, leaving the
  change-driven fan-out — for now — by hand. The skill's procedures catch up
  with the verbs: scaffold leads with `trellis scaffold`, escalations with
  `trellis escalate add/resolve`, a new lifecycle paragraph names
  `plan release/claim/block/unblock/retire`, and the lint-checklist section
  is two lines pointing at `trellis lint` instead of a prose re-enumeration.
  The two remaining manual sweeps are chartered as draft plans
  (`plans/sweep-derivation-command.md`, `plans/metrics-diff-command.md`) —
  the same extraction 0045 made for readiness, awaiting their own release.

- **Default complexity tiers retuned one notch down:** `mechanical` is
  `opus:medium` (was `opus:high`), `standard` is `opus:high` (was
  `opus:xhigh`); `deep` stays `fable:xhigh`. Budgets unchanged. A measured
  `standard` act spent xhigh-effort reasoning on work its own plan called a
  one-seam fix; the tier's meaning is the spec's, these prices are the
  binding's, and instances retune in `runtime.toml` as ever (decision 0032).

### Fixed

- **Readiness item 7 reads the plan above `## Escalations`, not the trail below
  it.** The deferral scan matched its tokens across the whole body, escalation
  records included — and a record raised about an unset done criterion
  necessarily quotes the deferral it reports. `conventions.md` keeps records
  verbatim and never deletes them, so answering the escalation could not clear
  the item: the quotation stayed, the scan kept failing, and since decision 0045
  that is a permanent dispatch hold. The plan was held for having reported the
  defect it then fixed (observed live: two plans in one domain, one of them the
  plan whose own escalation is quoted). The scan now stops at the
  `## Escalations` heading; a deferral anywhere above it still fails the item,
  and the passing detail says when a trail was excluded.

- **A `#anchor` ref resolves to a first-column table row, not only a
  heading.** The kernel's own template defines metrics as rows of the
  definitions table and the plan schema points `metrics:` refs at them, but
  `refs::resolve` matched headings only — so every domain that followed the
  template's format failed lint item 4 and readiness item 3 on refs that were
  correct, which the 0045 precheck then turned into held plans (observed
  live: 52 plans in one domain, one table). The eval fixtures never caught it
  because they define metrics as headings — both shapes resolve now.

- **The herdr backend no longer mistakes a dropped prompt for a finished
  turn.** Observed live: herdr accepts `agent.prompt` while Claude Code is
  still starting up, and the injected text vanishes — the pane settles `idle`
  over an empty input, `state_change_seq` moved by startup flaps alone, and
  both the tick loop and `--once`'s drain read that as a turn that ran to
  completion between observations. The session was retired `exit 0`, its
  workspace closed under `retain = "on-failure"`, and the plan bookkept as
  advanced when no agent ever saw it. A settle from a submitted-but-never-seen-
  working session is now believed only when the prompt's echo is in the pane's
  scrollback; otherwise the prompt is resubmitted (twice, with a beat for the
  startup window to pass) before the session is written off as lost — written
  off, not succeeded, so the workspace stays on screen and the run records a
  failure. The same machinery absorbs a spawn whose prompt is still refused
  `agent_not_ready` after the ten-second retry window (a concurrent cold start
  outlasts it): the session is handed over with the prompt unplaced instead of
  torn down. The cost is honest: a resubmission that raced a slow first
  injection can run the act twice, which the act prompt already absorbs by
  being idempotent — the duplicate turn reports "nothing to do" and settles.

### Added

- **`trellis view plan-graph`:** the `awaits:` DAG has been derived since
  sequencing landed, but every surface showed it one edge at a time — `plan
  list --held`, `readiness`, the board's dwell flag — so a chain's length, or
  one unretired plan holding five others, was invisible. The new view renders
  it as a Mermaid flowchart (canonical at `metrics/actuals/plan-graph.md`),
  arrows running target → dependent so the figure reads forward in time: what
  clears when. Red marks what lint already complains about — an `awaits:` cycle
  (item 22) or a target naming no plan (item 4), drawn as a ghost node rather
  than a silently dropped edge. Every live plan is a node, sequenced or not: the
  graph is the standing picture of the board, and a plan under no ordering
  constraint is a fact worth seeing. Retired plans are the one exclusion — they
  hold nothing and wait on nothing, so drawing them would bury the live picture
  under the whole history, and an edge onto a retired target drops with it,
  which is the same statement as its hold having cleared. Styling is
  stroke-only — no `fill`, no `color` — so the
  figure reads under both the light and the dark renderer theme, with status in
  stroke width and dash pattern as well as hue. Mermaid over DOT because it
  renders inline wherever the rest of `metrics/actuals/` is read, with no
  graphviz binary in the loop.

- **Decision hatching (spec v18; decision 0044):** `decisions/` is the one
  kind with no exit — append-only by rule 6, barred from the tier by rule 13 —
  and until now supersession was prose-only, so liveness was uncomputable and
  a domain could accumulate standing guidance that existed nowhere but the
  trail. The model now says decisions are memory, not manuals, twice over.
  Standing guidance an accepted decision establishes lands, in the same
  change, in the artifact that governs the behavior — `conventions.md`, a
  mandate, a context's README or `contracts/`, a rituals row — citing
  `(decision NNNN)`, so operating never requires reading the pile; a decision
  establishing nothing standing declares `Standing guidance: none.`
  Supersession becomes structural: decisions gain their first schema block
  (codifying the `status: accepted` the gate already keyed on) with an
  optional `supersedes:` list on the successor — the frozen target is never
  touched, a decision is live iff no accepted decision supersedes it, and
  partial influence stays prose.

  New: lint item 26 (supersession edges resolve — never self, never a
  non-decision, globs match exactly one, no cycles; dangling decision refs in
  authored bodies; and the unreachable set — accepted, live, uncited, not
  inert, unregistered — reported as a judgment queue, never violations, so
  adoption cannot flood a domain over its own history), `trellis view
  decisions` (the register: live / superseded-by / reachable-via, canonical at
  `metrics/actuals/decision-register.md`), a `## Decision registry` in
  `conventions.md` for prose-era dispositions recorded once, a
  derivation-sweep clause escalating orphaned operative content, and the
  skill's one-time "Compact a decision pile" procedure. Consolidation by
  restatement inside `decisions/` was the rejected shape: a frozen digest
  re-accumulates on its first correction, and the editable, owned guidance
  artifact gets the same consolidation with git history as its trail.

- **A terminal tier: `archive/` (spec v17; decision 0042):** a root accumulates
  work that is over, and until now there was nowhere for it to go. `plans/` was
  the sharp case — the only kind that grows without bound, has a terminal state,
  and has no ordering pushing dead entries away, since `decisions/NNNN-`
  self-sorts by age while plan slugs are semantic by mandate. The board and
  `trellis plan list` answer *what is live?*, and that is a reading: it does not
  shrink the tree a human scrolls or an agent globs. Worse, most kinds could not
  state the problem at all — only plans and strategies carried a terminal value,
  which is why the spec prescribed "re-parent or archive" in five places and
  defined archive nowhere.

  Rule 7 gains the line its refusals were already drawing: **a path may encode
  only what is fixed for an artifact's life** — kind, provenance — never a state
  it can leave, because a status directory makes every transition a move and
  every ref a forwarding address. Rule 13 then takes the one transition that
  never reverses. `archive/` mirrors the live hierarchy at `archive/{live
  path}`; admission is terminal-only (`retired`, `discarded`, or a new
  `status: archived` for kinds whose lifecycle had no terminal value); **the live
  path stays the address**, so an `awaits:` edge or an append-only decision keeps
  resolving across the move; and filing is a separate act from finishing, on a
  retention horizon declared in `conventions.md` and run by the steward's new
  archive sweep. Archived artifacts stay governed — lint, refs, census — and
  leave attention: dispatch, CODEOWNERS, and `plan list` skip them
  (`--archived` asks for them back). A tier is not a grouping because it invents
  no axis to browse by: you follow a ref into it, or you never look.

  **The load-bearing change is in `gitio`, and it is why this was checked before
  it was decided.** `commits` ran `git log` with no `--follow`, so a rename
  truncated the status timeline at the move: the board would see a one-entry
  timeline, `closed` would never increment, and closure rate and cycle time would
  read zero and `—` forever — the flow reading the Execution health pattern
  specifies as "the history of `plans/`", destroyed by the very artifacts it is
  computed from. `--follow` also cannot be paired with `--reverse`, which
  silently truncates the same walk, so the reverse happens in Rust. The tests
  assert the readings are *identical* before and after a move rather than merely
  that the file moved, which is the property the design exists to preserve.

  New: `trellis archive <artifact>` and `--sweep [--dry-run]`, lint item 25 (the
  tier mirrors the hierarchy, admits only terminal artifacts, never holds
  `decisions/`, and is never named by a ref), and `trellis plan list
  --archived`. Purge — `git rm` on the same sweep — was the simpler design and
  was refused: a retired plan's verdict is operational memory an agent should be
  able to grep, and `git log -S` is not a path anything reaches for unprompted.

- **Sessions talk back (non-normative; decision 0041):** the spawn was one-way
  — argv in, an exit code out — so a session that met an underspecified plan
  could only block it, write an escalation record, and exit, spending a full
  dispatch cycle on an answer that was often one sentence. The daemon now
  serves MCP on the socket it already listens on and hands each session its own
  endpoint as one more argv element, so a session can `trellis_ask` a question
  with options, `trellis_await` the answer, and `trellis_progress` a note the
  operator can watch without opening a gitignored log. **`trellis inbox` is the
  surface that answers** — `inbox` lists what is waiting, `inbox answer
  <ticket> <choice>` resolves it (a bare number picks that option), `--watch`
  blocks until something asks. The board grew a sessions strip and an answer
  form over the same state, and is deliberately *not* the contract: it is a
  plan view, and a question raised by a ritual or by the dispatch scan has no
  card to appear on.

  This is what shipped instead of a client-to-agent protocol. ACP was evaluated
  and refused with a pull-trigger: Claude is adapter-mediated, so the reference
  binding would gain an undeclared Node dependency; ACP has no budget
  expression and its model and effort controls are agent-defined option lists,
  so 0032's complexity map would get harder rather than portable. MCP is native
  everywhere, so the channel costs no adapter, no async runtime, and no second
  listener — and a template that omits `{mcp}`, or a daemon run with
  `--no-http`, runs exactly as before.

  Two consequences worth naming. The daemon gains its first non-GET routes —
  `/mcp/{token}` and `POST /api/sessions/{token}/answer` — and the read-only
  claim in `server.rs` is amended in the same commit rather than left standing:
  the *tree* stays read-only, both routes write only `.trellis/runtime/`, and
  neither is ingress, because the session they concern is already running under
  a mandate. And where there is no socket to call back on — `--no-http`, or
  `--once`, which is a cron pass with no operator watching — the channel is off
  and the daemon says so once, rather than refusing to start.

- **`trellis serve` stops the sessions it started:** `SIGTERM` now cancels
  in-flight sessions, waits for them, and records the exits, instead of exiting
  and orphaning children that are process-group leaders by design. `SIGINT`
  keeps its default deliberately — a stray Ctrl-C still must not kill a session
  mid-artifact, which is why the process groups exist.

- **`trellis tree` — the artifact census (non-normative; decision 0040):** the
  companion the discovery-scope work needs. Every other read command answers a
  question about an artifact you already know the path of; nothing answered
  "which files does the kernel consider mine?", so the only way to find out was
  to read the findings and infer it from what did *not* appear. `trellis tree`
  draws the artifacts as a tree with each one's kind, owner, and status, and
  prints the same `scope` report lint does — because a census that showed only
  what it found would answer half the question, and the half it omitted is the
  one behind "why is that README being linted?". `--format json` gives the same
  data machine-shaped, `--flat` gives one path per line for piping, and
  `--kind` plus path positionals narrow it (segment-matched, so `tree org`
  means the directory and not every path starting with those letters). The row
  shape lives in `facts.rs` beside the plan census, so the daemon can serve it
  without a second encoding.

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

- **Artifact discovery is declared, not inferred (non-normative; decision
  0040):** `Tree::load` answered "what is an artifact?" with one rule — every
  `.md` under the root outside a dot-directory — which is less a heuristic than
  the absence of one, and which `spec/model.md` has never described: it puts
  real code inside a root at `solution/{bc}/{deployment unit}/`, and code brings
  markdown belonging to its own project. A JavaScript deployment unit's
  `node_modules/` was swept in whole, firing lint item 1 ("no frontmatter") on
  every dependency README with `owner: None` — an escalation with no addressee —
  while item 10 byte-read every file in it and, since 0038, the daemon re-walked
  it on every scheduling pass. `conventions.md` compounds it: the boundary
  guarantees name submodules and vendored copies as tools for imported material,
  so a root is *expected* to hold other people's repositories. The `paths`
  positional was never the remedy — it retains findings *after* every rule has
  run over the whole tree. **Three rules now bound the walk, two declared and one
  structural, and the differences between them are the point.** `.gitignore` says
  a path is not in this repository. **A directory holding its own `.git`** says it
  is a *different* repository — a submodule (a `.git` file), a nested clone or
  worktree (a `.git` directory); this one cannot be a declaration, because a
  submodule enters the parent's index as a single gitlink and never as its
  contents, so no ignore list can name it and no `.gitignore` discipline will
  hide it. Both are pruned during traversal, so nothing sees them, item 10
  included: one is what git refuses to store, the other belongs to a repo with
  its own owner and is materialized read-only here anyway (`.env` has always been
  invisible under the dot rule; this is consistent, not new). `conventions.md`'s
  new **carried-content registry** — a third registry beside plan-types and tags,
  read by the same `registry_items` machinery, so no new parsing — covers what
  neither structural rule can: a path committed into *this* repo but authored
  elsewhere, whose markdown stops being an artifact while item 10 keeps sweeping
  it, precisely because it *is* tracked here. Symlinked material needs no rule;
  the walk never followed symlinks, which is now true on purpose. All three hold
  in `Tree::load`, so lint, dispatch, views, readiness, and the daemon inherit
  them together. The scope is reported and never silent — `--format json` carries
  a `scope` object, the text format one line — because a narrowing nobody states
  reads as "everything was checked", and the honest cost here is that a
  `.gitignore` line can hide an artifact with no violation to show for it.
  Nothing normative moves: which files a tool considers is a binding concern, and
  obliging every future binding to implement this mechanism is exactly what
  keeping it out of `spec/model.md` avoids.
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
