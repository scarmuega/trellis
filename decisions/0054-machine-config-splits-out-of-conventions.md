---
provenance: authored
status: accepted
date: 2026-08-14
---
# 0054 — Machine config splits out of conventions.md

## Context

Every root carried `conventions.md`: a hand-copied fork of the spec's
normative text, annotated per instance and hand-merged as the spec moved.
Eight dogfood roots showed how that model fails. No two copies matched. One
was frozen at a pre-`ready` plan lifecycle several versions stale — and since
every session is told the instance's copy is authoritative, its agents loaded
*false* rules with full authority; a copy does not degrade into missing
information, it degrades into wrong information. The most-customized root
required manual three-way merges of normative prose against a moving
template, per domain, forever.

The kernel's reads made the file's second failure visible: the retention
horizon was scraped out of free prose by regex (`archive after … N days`),
the four registries by heading-slug convention, and the carried-content list
by a second, independent parse path that read the file straight off disk
before the tree existed. Prose bent to fit a regex ("terminal artifacts that
have been **archive after 90 days**") is neither good prose nor good config.

The file fused three things with different fates: restated spec (~75%),
machine-read config, and genuine instance prose.

## Decision

**The three parts split, and each goes to its authoritative home.**

`trellis.toml` — new, domain-owned, machine-read, and the root marker in
`conventions.md`'s place. It carries the required `spec` pin, the plan-type /
tag / decision registries (name → one-line definition; the kernel consumes
keys, agents read values), the `carried` prefixes, and the
`archive.after_days` retention horizon. Parsing is schema-validated with
unknown keys refused, mirroring `runtime.toml` — but unlike `runtime.toml`,
absence is never legal (it is what makes the directory a root) and a file
that does not parse refuses the whole load with a hard error rather than
shrinking to defaults: a lint pass over empty registries would report
violations that are not there, with deterministic authority (decision 0037).
Root *discovery* stays existence-only, so a corrupt file still marks a root
and the write gate keeps working while the first real command says what is
wrong. Lint item 18 moves off its regex over `decisions/0000-adopt-trellis.md`
and owns exactly the mismatch case, on `trellis.toml`.

`domain.md` — the renamed remainder, optional, holding only what prose alone
can: boundary guarantees, secrets policy, the runtime-binding choices, and
the citation trail (its runtime-binding line cites `decisions/0000`, which is
what keeps a fresh root's trail reachable under item 26 — deliberately not a
`[decisions]` entry, because the registry means "disposition judged", not
"cited"). The name no longer claims to carry the conventions, because it
does not.

The restated spec text is deleted, not relocated per-instance: the spec
(`spec/model.md` for schemas, `spec/runtime.md` for the escalation-record
schema, the kernel for the mechanical rules) is authoritative, and a root
declares which version it operates against rather than carrying a drifting
transcript of it. Re-pinning becomes the explicit, lint-visible act of
adopting the intervening changes.

The cutover is hard: no dual-read path, no `trellis migrate` command. The
kernel is pre-1.0 with every known instance in reach; existing roots are
migrated by hand (an agent's errand), and an unmigrated root fails root
detection with the new marker set named.

## Consequences

Spec v20 → v21, breaking: the hierarchy replaces `conventions.md` with
`trellis.toml` + `domain.md`, and the root marker set changes in
`root.rs`, `hooks/gate.mjs`, and every prose contract that spells it. The
kernel gains `domain.rs` (the `DomainConfig` loader), `Tree` carries the
parsed config, `Scope` takes its carried list from it, and
`markdown::registry_items` — the heading-slug registry scraper — is deleted
with the `archive after` regex. `Kind::Conventions` becomes `Kind::Domain`.
Items 4, 8, 24, and 26 retarget their messages; no item is added, removed,
or renumbered.

Instances lose the ability to restate rules with local wording — a deviation
is now a decision record or a spec change, never a quietly divergent copy.
The trade accepted: a bare clone no longer carries the full normative text;
it carries the pin, and the spec travels with the kernel and plugin.

Standing guidance: `spec/model.md` (hierarchy, rule 6), the conventions-lint
checklist (Scope, items 4/8/18/26), the template, and the `trellis:conventions`
skill.

## Alternatives rejected

- **Keep the prose copy, add a generated rendering** (spec@pin + config +
  local overlay, refreshed by the derivation sweep). Drift-proof by
  construction, but it keeps two homes for one text and adds a generator to
  maintain; nothing yet demands an in-repo full transcript. Revisitable
  without unwinding this decision.
- **A `trellis migrate` command.** One-shot code for a one-shot event across
  eight known roots; the recipe is simpler as an agent's errand than as a
  maintained subcommand.
- **Parse errors as lint findings.** A lint that runs over a config it could
  not read reports wrong findings deterministically — the exact failure mode
  0037 exists to prevent.
- **`conventions.md` stays the marker with `trellis.toml` beside it.** Keeps
  the old name pointing at a file that no longer holds the conventions, and
  makes the prose file load-bearing when it is the one part that should be
  optional.
