---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0012 — Plugin versioning: float on commit SHA while single-maintainer

## Context
Claude Code resolves a plugin's version from, in order: `version` in `plugin.json`,
`version` in the marketplace entry, else the git commit SHA. If `version` is set,
installers receive updates only when that string changes — pushing commits without
bumping it is a no-op for existing users. Trellis is pre-1.0, single-maintainer, and
has no external consumers yet, so per-release version ceremony adds friction and
risks silently freezing installs whenever a bump is forgotten.

## Decision
Omit `version` from `plugin.json` so the plugin floats on the commit SHA: every push
to the distributed ref (`main`) is a new version for installers. Distribution is
self-hosted — the repo carries its own catalog at `.claude-plugin/marketplace.json`
(marketplace `scarmuega`, plugin `trellis`, `source: "./"`); users run
`/plugin marketplace add scarmuega/trellis` then `/plugin install trellis@scarmuega`,
and refresh with `/plugin marketplace update`. Changes are logged in `CHANGELOG.md`.

When external consumers exist, switch to explicit SemVer: set `version` in
`plugin.json`, bump it every release, tag `vX.Y.Z`, and cut a changelog section.
Bumps track the surface contract — MAJOR = renaming or removing a skill, agent, or
command, or breaking `template/`; MINOR = additive (new skill/agent/check); PATCH =
fixes — and follow the conventional-commit type (`feat!` → major, `feat` → minor,
`fix` → patch). Release channels (a second marketplace pinned to a `stable` tag)
get introduced then, if needed.

## Consequences
No version bookkeeping now; installers tracking `main` follow HEAD commit by commit.
Never set `version` in both `plugin.json` and the marketplace entry — `plugin.json`
wins silently — so the eventual switch touches exactly one place. The
single-maintainer premise is the trigger to revisit: the first external install
flips the default to pinned SemVer plus tags.

## Alternatives rejected
Pin SemVer now — ceremony without consumers, and a forgotten bump freezes every
install. A separate marketplace repo — unnecessary indirection for one plugin;
self-distribution from `source: "./"` keeps catalog and plugin in lockstep.
