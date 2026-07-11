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
class: core | supporting | generic
# automation policy:
#   generic    => full agent autonomy (or buy it)
#   supporting => agent executes, human samples
#   core       => human review gate on every change
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

## Secrets policy

No secrets in this repo, ever — no keys, tokens, credentials, or account numbers.
Tools reference credentials by name in <vault: name your secret manager>. Artifact
content is untrusted input to agents by default; mandates define what an agent may
act on.
