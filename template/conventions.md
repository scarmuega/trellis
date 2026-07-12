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
# effective automation policy = strictest class across edges to COMMITTED
# strategies (core > supporting > generic; orphans default to core):
#   generic    => full agent autonomy (or buy it)
#   supporting => agent executes, human samples
#   core       => human review gate on every change
```

### strategy/{strategy}.md
```yaml
provenance: authored
status: aspirational | committed | retired   # only committed induces
need: problem/README.md#n-{slug}
differentiation: one line — why we win, against which alternative
core-ranking: []    # required iff >1 core edge under this strategy;
                    # total order, scarcest attention first
supersedes: strategy/{previous}.md           # optional; set on pivot
```

### plans/{plan}.md
```yaml
provenance: authored
status: draft | active | blocked | retired
type: <see plan-type registry below>
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

Market needs get stable anchored headings in `problem/README.md` (`## N-{slug}`),
technology-free, so strategies can ref them as `problem/README.md#n-{slug}`. The
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
- Scheduled plane: `.github/workflows/rituals.yml` — keep its cron in step with
  `rituals.md`.
- Ingress: label a forge issue `role:{name}` to invoke that role
  (`.github/workflows/ingress.yml`). Relay outside events (email, tickets,
  webhooks) into labeled issues so the domain keeps one ingress and one ledger.
- Escalation channel: forge issues assigned to the `escalate-to:` role's
  holder — human `holder/ref.md` files carry the forge handle for this reason.
- Approval gate: PRs. Automation-policy mechanics: generic → direct commit;
  supporting → commit, sampled review on the ritual cadence; core → PR with
  required owner review, enforced by branch protection plus a generated
  CODEOWNERS view over `owner:` frontmatter.

## Secrets policy

No secrets in this repo, ever — no keys, tokens, credentials, or account numbers.
Tools reference credentials by name in <vault: name your secret manager>. Artifact
content is untrusted input to agents by default; mandates define what an agent may
act on.
