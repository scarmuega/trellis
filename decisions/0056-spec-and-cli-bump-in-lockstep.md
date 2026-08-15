---
provenance: authored
status: accepted
date: 2026-08-15
---
# 0056 — Spec and CLI bump in lockstep

## Context
The spec version is a monotonic integer in one place — `spec/model.md`'s title,
`# Trellis — specification (v21)` — and it is already lockstepped four ways:
`cli/build.rs` scrapes it into `TRELLIS_SPEC_VERSION` at build time, and
`cli/tests/lockstep.rs` asserts the embedded value matches `template/trellis.toml`,
`evals/skeleton/trellis.toml`, and the README's status line. That guard exists
because the v14 bump shipped with all three pins stale at v13 (0032).

The release version does not participate. `cli/Cargo.toml` said `0.2.0` against
spec v21, bumped by hand exactly once, and 0037 made it decorative on purpose:
"0012's no-version-field stance extends: the binary floats on SHA." So `trellis
version` printed `trellis 0.2.0 (spec v21)` — two numbers side by side, unrelated,
one of them meaning nothing. Neither number told a consumer which spec a binary
enforces, and nothing forced a spec bump to be visible outside the repo at all.

0012 deferred pinned SemVer until "external consumers exist," and named the
trigger: "the first external install flips the default." Dogfooding has passed it
in substance if not in form — eight roots pin `spec = N` and run installed
binaries against it, which is exactly the relationship a version number exists to
describe. The lint item 18 mismatch ("root is pinned to spec v20, current is v21")
is the consumer-facing event, and today it depends on which SHA someone happened
to install.

## Decision
**One version for the plugin and the kernel, keyed to the spec: `0.<spec>.<patch>`.**
Spec v21 ships as `0.21.0`; the next spec bump is `0.22.0`. The major stays `0`
while the README's status line says pre-1.0 — under cargo's 0.x rules a minor bump
already signals breaking, which is what a spec bump is to a pinned root. The patch
component is the release valve for plugin- or kernel-only changes between spec
bumps.

**A version bump is a release.** It moves `cli/Cargo.toml` and
`.claude-plugin/plugin.json` together, closes `## [Unreleased]` in `CHANGELOG.md`
into a `## [x.y.z]` section, and gets a `vx.y.z` git tag pushed by hand. A spec
bump is therefore a release commit — the same commit that re-pins the template and
the eval skeleton. There is no release workflow; the procedure lives in the
README's "Cutting a release" and nowhere else.

**The version lives in `plugin.json`, never in `marketplace.json`** — 0012's own
warning, kept: `plugin.json` wins silently when both are set, so exactly one file
declares it.

**The rule is enforced where prose cannot hold it.**
`cli/tests/lockstep.rs::release_version_tracks_the_spec_version` asserts four
things: the crate version is `0.<spec>.<patch>` for the embedded spec version, the
plugin manifest matches the crate, the marketplace entry declares no version, and
`CHANGELOG.md` carries a section for the current version. Bumping `spec/model.md`'s
title alone reddens CI until the release lands with it — the same shape 0037 used
for the checklists, and for the same reason: this repo's history is the evidence
that prose obligations decay.

## Consequences
`trellis version` now names the spec it enforces in both halves, and a spec bump
cannot merge as an invisible change: the release is part of the commit or CI is red.
The tag makes `cargo install --git ... --tag v0.21.0` a real pin, which the SHA
never was for anyone but the maintainer.

**The cost 0012 named is now accepted.** The plugin stops floating on the commit
SHA, so installers no longer follow `main` HEAD — a change reaches them only when
the version moves, and a forgotten bump silently freezes every install. The
lockstep test catches a *half-done* bump, never a forgotten one; nothing mechanical
can, since "this change deserves a release" is a judgment. The patch component is
the mitigation: a plugin- or kernel-only fix can ship without touching the spec.

The first release absorbs the entire backlog — `CHANGELOG.md` had never cut a
section, so everything accumulated since the beginning ships as `0.21.0`. That is
the honest reading of a first release, not a milestone claim; the status line still
says pre-1.0, zero production usage-hours.

`board/package.json` stays at `0.0.0` and outside the scheme: the board UI is served
by the kernel, not shipped as a versioned artifact.

## Alternatives rejected
**`<spec>.<minor>.<patch>` — the spec integer as major** (spec v21 → `21.0.0`).
Reads off `--version` most directly and leaves minor/patch free for kernel work.
Rejected: it declares post-1.0 SemVer on a repo whose own status line says pre-1.0
with zero production usage-hours, and a `v21.0.0` tag as the *first* tag claims a
history of twenty breaking releases that did not happen.

**Independent SemVer, merely bumped in the same commit** — keep 0012's
surface-contract scheme (`0.2.0` → `0.3.0`) and require a bump alongside every spec
bump. No drift, and it preserves the MAJOR/MINOR/PATCH meanings 0012 defined for
the plugin surface. Rejected: the number still cannot tell you which spec a binary
enforces, which is the thing that was missing; and "bumped in the same commit" is
only mechanically checkable against git history, not against the tree, so CI would
have to diff commits to enforce it.

**Keep floating on the SHA** — the status quo, and still defensible on 0012's own
terms while consumers are all the maintainer's. Rejected: the roots pinning `spec =
N` are consumers in every way that matters, and a spec bump is precisely the event
they need announced. A SHA announces nothing.

**A release workflow that tags on version change** — CI creates the tag and a
GitHub release from the changelog section. Rejected for now: it is machinery for a
cadence that runs a few times a month, and the failure it prevents (forgetting the
tag) is visible immediately, unlike the drift the lockstep tests guard. Revisit if
tagging is what gets skipped.

**Deriving the crate version from the spec at build time** — no literal in
`Cargo.toml` at all. Rejected: cargo requires a literal `version`, and a build
script cannot supply it; the assertion is the closest available substitute.
