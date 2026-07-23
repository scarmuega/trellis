---
provenance: authored
owner: <owner>
---
# Agents

This repo is a Trellis domain: one business operated as a versioned filesystem of
markdown artifacts. The repo is the organization — agents load artifacts as
instructions, humans review diffs as governance.

**Read `conventions.md` first.** It is this domain's authoritative copy of the
schemas, registries, and policies, and overrides any general knowledge. The full
placement guide and procedures live there.

## Routing — where each kind lives

    market.md     needs and personas — the invariant layer, technology-free
    strategy/     committed solutions to needs — only the committed band induces
    problem/      the subdomains those strategies induce
    solution/     bounded contexts: contracts/, skills/, deployment units
    plans/        time-bounded execution — status + type
    metrics/      definitions.md (authored) + actuals/ (generated)
    decisions/    ADRs, append-only
    org/          roles: mandate.md (authority) + holder/ (identity)
    also: conventions.md · rituals.md · glossary.md · brand.md · economics.md

## Before you write

- Check `owner:` and `provenance:` in the target's frontmatter. Edit only what
  your role owns; never hand-edit `provenance: generated` — fix the generator.
- Decisions are append-only: supersede, never edit.
- Act under a role — `org/{role}/mandate.md` bounds your authority and spend.
- No secrets in the repo, ever — reference credentials by name.

If your harness runs the Trellis plugin, the `trellis:conventions` skill loads
this map and the schemas; `/trellis:act`, `/trellis:plan`, `/trellis:focus`, and
`/trellis:ritual` invoke roles and rituals.
