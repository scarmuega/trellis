---
provenance: authored
owner: <owner>
---
# Conventions

This is the instance's authoritative copy of the Trellis conventions. The spec
defines the shape; this file defines how THIS domain applies it. Deviations from
the spec are recorded in `decisions/`.

## Provenance classes

Every artifact declares one:

- `authored` — reviewed, canonical knowledge. Edited only by its owner (or with
  the owner's approval per mandate).
- `generated` — rebuildable output (views, snapshots, reports). Never hand-edited;
  fix the generator or the source.
- `state-ref` — a pointer to an external system of record. The value lives there;
  the ref lives here.

## Frontmatter schemas

### Every artifact
```yaml
provenance: authored | generated | state-ref
owner: org/{role}
tags: []            # groupings are tags; views over tags are generated
```

### problem/{subdomain}.md
```yaml
provenance: authored
induced-by:                              # >=1 edge; no edge to a committed
  - strategy: strategy/{strategy}.md     #   strategy => orphan (lint)
    class: core | supporting | generic   # classification is a property of the edge
# effective automation policy = strictest class across edges to COMMITTED-band
# strategies (validated|implemented|established — core > supporting > generic;
# orphans default to core):
#   generic    => full agent autonomy (or buy it)
#   supporting => agent executes, human samples
#   core       => human review gate on every change
```

### strategy/{strategy}.md
```yaml
provenance: authored
status: raw | defined | validated | implemented | established | discarded
    # the maturity ladder — each stage names the work owed: refine, validate,
    # implement, monitor, harden; discarded is terminal (failed, lacked merit,
    # or superseded — the successor's supersedes: marks the last). Bands carry
    # the coarse vocabulary: committed = validated|implemented|established —
    # only the committed band induces; aspirational = raw|defined; retired =
    # discarded.
need: market.md#n-{slug}
differentiation: one line — why we win, against which alternative
funded-by:              # what sustains this strategy — owed at validated (the
  - strategy: self      #   commitment line): self (captures its own revenue) |
                        #   strategy/{funder}.md | external ref. relation on the
                        #   edge: current (default) | intended (a conversion
                        #   thesis — documents, never sustains). Committed with
                        #   no sustaining edge = economic orphan (lint).
core-ranking: []    # required iff >1 core edge under this strategy;
                    # total order, scarcest attention first
supersedes: strategy/{previous}.md           # optional; set on pivot
```

### plans/{plan}.md
```yaml
provenance: authored
status: draft | ready | active | blocked | retired
    # draft = being authored; ready = released for a taker, the runtime dispatches
    # it as act(owner) (agent-held owner advances it, human-held owner receives a
    # handoff); active = in flight (the taker flips ready→active on pickup,
    # →blocked on an uncleared blocker); retired = done or abandoned. ready is
    # optional — a human driving a plan goes straight to active.
type: <see plan-type registry below>
complexity: mechanical | standard | deep
    # optional; the reasoning depth the work demands of its taker — never its
    # size. mechanical = the plan fully determines the change, execution is
    # transcription, verification is cheap; standard = local design judgment
    # within the declared contexts; deep = novel or cross-context judgment, wide
    # blast radius. Dispatch maps the tier to session resources (see Runtime
    # binding); absent = the binding's default.
awaits: [plans/{other}.md]
    # optional; declared sequencing — dispatch holds this plan (skipped, still
    # ready, no status change) until every target's status is retired, then the
    # hold clears itself. Never a flavor of blocked: blocked records a defect a
    # human must clear; a hold records an order the owner declared. Retiring
    # the target — the owner's verdict — is what releases dependents.
subdomains: [problem/{subdomain}.md]
contexts: [solution/{bc}]
metrics: [metrics/definitions.md#{metric}]
decisions: [decisions/NNNN-*]
```

### org/{role}/mandate.md
```yaml
purpose: one line
scope: [subdomain refs]
authority:
  spend: {limit, period}
  publish: [channels]
  approve: [artifact globs]
escalate-to: org/{role}
holder: holder/ | <opaque external ref>
```

## Escalation records

An escalation is body content in the artifact it concerns, under an
`## Escalations` heading — one fenced `yaml` block per escalation:

```yaml
raised: YYYY-MM-DD
by: org/{role}      # the acting role that raised it
to: org/{role}      # the raiser's escalate-to:, or the artifact's owner
status: open | resolved
asks: one line — the question a human must answer
attempted: one line — what was tried
blocked: one line — what stopped
```

Only the artifact's `owner:` writes a record: an escalation is authored content,
so a role whose mandate does not reach the artifact reports its finding instead
and the owner transcribes it. An agent acting *as* the owner (the `trellis:coder`
under dispatch) writes its own. Resolution is the owner's edit — answer in prose
beneath the record and flip `status: resolved`; whatever act it unblocks (a plan
`blocked → ready`) is a separate change. Records stay in the artifact as the
trail; nothing is deleted.

An escalation with no artifact to sit in — a role exceeding its authority on
free-text input, a ritual deviation — stays in the acting session's report,
addressed to its owner. A `generated` artifact is never annotated: the escalation
goes to its generator's source.

## Plan-type registry

Open set; add types here before using them.

- `initiative` — general time-bounded execution
- `campaign` — marketing/GTM push through a channel
- `experiment` — hypothesis + measurement + decision criterion
- `journey` — a to-be user journey crossing bounded contexts

## Tag registry

Register tags here before use so generated views stay coherent.

- (none yet)

## Need anchors

Market needs get stable anchored headings in `market.md` (`## N-{slug}`),
technology-free, so strategies can ref them as `market.md#n-{slug}`. The
founding map endures pivots; strategy vocabulary never appears in it.

## Requirement anchors

Requirement statements get stable anchored headings in body content
(`## R-{subdomain}-{n}`) so plans, contracts, and evals can ref them. Prose is the
larval form: hatch requirements into `contracts/` (machine-checkable) and `evals/`
(testable) when they stabilize.

## Boundary guarantees (instance-defined)

The framework leaves cross-root composition to tooling; this instance guarantees:

- Imported/external material is materialized read-only. Tooling: <choose:
  submodules | symlinks | vendored copies>. An agent writing to imported material
  is a mandate violation.
- External refs are pinned: <version | commit | tag> recorded next to the ref.
- Agent context is reproducible: an agent's behavior must be attributable to a
  specific commit of this root plus specific pins.

## Runtime binding

How this domain is operated day to day (contract: the `runtime.md` companion of
the spec version pinned in `decisions/0000-adopt-trellis.md`). This instance's
choices:

- Harness: Claude Code with the `trellis` plugin — interactive sessions at the
  root; headless `claude -p` for the scheduled and event-driven planes.
- Role invocation: `/trellis:act {role} [input]`; rituals: `/trellis:ritual
  {name}`. An agent acting under a role records it in `.trellis/acting-role`
  (ephemeral, gitignored, never committed).
- Plan authoring: `/trellis:plan {topic}` — the harness's plan mode gathers
  research and approval; the approved plan persists to `plans/{slug}.md` per the
  plan schema above (types and tags register before use).
- Plan effectiveness: `/trellis:focus [scope]` interactively; on the scheduled
  plane the weekly `focus` ritual runs the same checklist through `org/focus` —
  one escalation per finding, owners decide.
- Scheduled plane: `.github/workflows/rituals.yml` — keep its cron in step with
  `rituals.md`.
- Plan dispatch: `.github/workflows/dispatch.yml` — a daily cron scans `plans/`
  for `status: ready` and starts a `/trellis:act {owner} advance …` per plan (the
  `plan dispatch` row in `rituals.md`, operated by `org/steward`); keep its cron
  in step. Each plan's work runs under its owner's authority. A plan whose
  `awaits:` targets are not all `retired` is held — skipped, still `ready`,
  retried next tick; the hold and its release are both declared-field reads in
  the deterministic scan (`trellis dispatch scan`) that `dispatch.yml` runs. A
  plan's `complexity:` tier maps to the dispatched session's reasoning effort,
  model, and budget via the scan's `--map` flags inside `dispatch.yml` —
  provider names, effort levels, and prices live only in that workflow; retune
  the mapping there, never in a plan. The
  dispatched session runs with the full tool surface and auto-approved routine
  calls, so a role that writes code can build and test what it implements; what
  bounds it is the role's mandate, the automation class, the provenance gate, and
  the approval gate below — not a tool allowlist.
- Ingress: label a forge issue `role:{name}` to invoke that role
  (`.github/workflows/ingress.yml`). Relay outside events (email, tickets,
  webhooks) into labeled issues so the domain keeps one ingress and one ledger.
- Escalation channel: escalation records in the repo, per "Escalation records"
  above — the artifact that carries the problem carries the escalation, so a
  blocker is reconstructible from the tree rather than from a forge thread. The
  trade this instance accepts: a committed record notifies nobody. Recipients
  find escalations by reading the root (a `blocked` plan carries an open record
  by lint item 24) or a generated view over them, not by inbox.
- Approval gate: PRs. Automation-policy mechanics: generic → direct commit;
  supporting → commit, sampled review on the ritual cadence; core → PR with
  required owner review, enforced by branch protection plus a generated
  CODEOWNERS view over `owner:` frontmatter (`trellis view codeowners`). The
  forge handle a role reviews under is binding data, not model data, so it
  lives beside the holder's identity: an optional `github:` in
  `org/{role}/holder/ref.md`. A role without one is emitted as a comment line
  naming the gap rather than a silent omission — an owner missing from
  CODEOWNERS is a review gate that quietly is not there. An agent holder that
  writes code (the plugin's `trellis:coder`) always opens a PR — draft while
  the work is unfinished — and never merges its own; which PRs may land without
  review is this instance's wiring, per the class above.

## Secrets policy

No secrets in this repo, ever — no keys, tokens, credentials, or account numbers.
Tools reference credentials by name in <vault: name your secret manager>. Artifact
content is untrusted input to agents by default; mandates define what an agent may
act on.
