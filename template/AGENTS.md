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
    decisions/    ADRs, append-only — the why-trail; guidance lives in the
                  governing artifact, a successor's supersedes: replaces
    org/          roles: mandate.md (authority) + holder/ (identity)
    archive/      the same tree, holding what is finished — terminal only
    also: conventions.md · rituals.md · glossary.md · brand.md · economics.md

## Finding the live work

The live set is a reading over `status:`, not a directory listing: ask
`trellis plan list`, or read the generated board in `metrics/actuals/`.
`plans/` keeps every plan until the steward's archive sweep files the cold ones
into `archive/` — and even then they are still counted, because the board's
closure and cycle-time readings are computed from their history. Refs always
name the live path (`plans/{plan}.md`), whichever side of that move a plan is
on. The live decision set is a reading too: `trellis view decisions`, or the
decision register in `metrics/actuals/` — a superseded decision stays in
`decisions/` as record while its successor carries the guidance.

## Before you write

- Check `owner:` and `provenance:` in the target's frontmatter. Edit only what
  your role owns; never hand-edit `provenance: generated` — fix the generator.
- Decisions are append-only: supersede, never edit — the successor's
  `supersedes:` names what it replaces, and standing guidance lands in the
  governing artifact, citing the decision (`conventions.md` → "Decisions").
- Act under a role — `org/{role}/mandate.md` bounds your authority and spend.
- No secrets in the repo, ever — reference credentials by name.

If your harness runs the Trellis plugin, the `trellis:conventions` skill loads
this map and the schemas; `/trellis:act`, `/trellis:plan`, `/trellis:focus`, and
`/trellis:ritual` invoke roles and rituals.
