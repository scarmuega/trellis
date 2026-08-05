---
provenance: authored
status: accepted
date: 2026-08-05
---
# 0040 — Artifact discovery is declared, not inferred

## Context
`Tree::load` has always answered "what is an artifact?" with one rule: every
`.md` under the root that is not inside a dot-directory. That rule is not a
heuristic so much as the absence of one — no `.gitignore` awareness, no ignore
file, no size or depth bound. It held while roots were markdown and nothing
else.

`spec/model.md` has never described such a root. It puts real code inside one by
design: `solution/{bc}/{deployment unit}/` is where a bounded context's
deployment units live, and `org/{role}/holder/tools/` is where an agent package
keeps its own. A deployment unit written in JavaScript has a `node_modules/`; one
in Rust has a `target/`; both carry thousands of markdown files that belong to
their projects and to nobody in this domain. Discovery swept all of it in.

`conventions.md` compounds it. Its boundary guarantees name three tools for
imported material — submodules, symlinks, vendored copies — so a root is
*expected* to hold other people's repositories. Symlinks were already safe by
accident (the walk does not follow them). The other two were not, and a submodule
is the worst case of the three: its files are not in this repo's index at all,
only a gitlink is, so no ignore list will ever name it and no amount of
`.gitignore` discipline will hide it.

Three costs, and only the first is the one anybody reports. **Noise**: every
dependency `README.md` fires lint item 1 — "artifact has no frontmatter" — with
`owner: None`, which is an escalation with no addressee, so the finding cannot
even be routed. **Cost**: item 10's secrets sweep byte-reads every non-markdown
file up to 512 KB, and `Tree::load` reads every markdown file into memory.
**Cadence**: since 0038 the daemon loads the tree on every scheduling pass —
sixty seconds by default — so a dependency tree is not walked once per lint but
continuously.

The `paths` positional on `trellis lint` looks like a remedy and is not: it
retains findings by prefix *after* every rule has run over the whole tree. It
narrows the report, never the walk.

Underneath the symptom is a modelling gap. The root's *tree* and the domain's
*artifacts* were the same set, and the model has outgrown that. A root carries
material it does not author, and nothing in the tree says which is which.

## Decision
**The artifact boundary is stated, never guessed at** — and three rules carry it,
because a root holds foreign markdown for three different reasons. Two are
declarations the domain makes; the third is a fact about the filesystem that
needs no declaring.

- **`.gitignore` — "not in this repo."** `Tree::load` prunes git-ignored paths
  during traversal, via `git ls-files --others --ignored --directory
  --exclude-standard`. `--directory` collapses a wholly ignored tree to one
  prefix, so the prune set is a handful of lines rather than forty thousand
  paths — and git reports the *largest* wholly-ignored prefix, narrowing it the
  moment the directory holds anything not ignored, so pruning a prefix never
  prunes a file that was not itself excluded. Outside a git repo the list is
  empty and behavior is exactly what it was.
- **A directory holding its own `.git` — "a different repository."** Pruned on
  sight. A submodule's `.git` is a file, a nested clone's or worktree's is a
  directory; `.exists()` catches all three for the cost of a `stat`, with no
  subprocess and no registry entry. This rule is not a convenience: a submodule
  *cannot* be expressed as an exclusion, because it is present in the parent's
  index as one gitlink and absent from every file listing git can produce. The
  first rule is structurally incapable of covering it, which is why this is a
  rule and not a footnote to that one.
- **`## Carried-content registry` in `conventions.md` — "in the repo, not
  ours."** Root-relative path prefixes, a third registry beside the plan-type
  and tag registries, read by the same `markdown::registry_items` machinery and
  therefore adding no new parsing. It covers what the first two structurally
  cannot: material the domain *commits into its own repository* but does not
  author — a vendored copy, a deployment unit's own docs, an imported corpus.

**The three are enforced at two points, and the difference is the decision.**
Git-ignored paths and nested repositories are pruned from the walk outright:
nothing sees them, item 10 included, because neither is in the repository the
secrets policy speaks about — one git refuses to store, the other belongs to
another repo with its own owner and its own policy, materialized read-only here
so this domain could not fix a finding in it anyway. Carried paths are still
walked and still swept — they *are* tracked here — and merely stop being
artifacts: items 1, 2, 8 and 24 go quiet over them while item 10's reach is
untouched.

Symlinked material, the third tool the boundary guarantees name, needs no rule:
the walk does not follow symlinks, so a symlinked tree is already outside it.
That was true before this decision and is now true on purpose.

The scope is reported, never silent. `trellis lint --format json` carries a
`scope` object naming all three lists; the text format prints one line when any
is non-empty. A narrowing nobody states reads as "everything was checked."
**`trellis tree`** exists for the same reason: a census of what discovery took
in, drawn as a tree with each artifact's kind and owner, carrying the same scope
report — so "what does the kernel consider mine?" is a question with a direct
answer rather than one inferred from which findings did not appear.

Both declarations live in `Tree::load`, so they hold for every consumer of the
tree — lint, dispatch, views, readiness, and the daemon — rather than for lint
alone. The signature is unchanged; the four call sites are untouched.

## Consequences
A domain whose bounded contexts hold working code gets a lint that reads its
markdown and not its dependencies', on the strength of a `.gitignore` it already
maintains. Nothing is asked of existing roots for the common case; the registry
is opt-in and starts empty. The daemon stops re-walking dependency trees every
minute, which was the largest recurring cost and the least visible.

**Item 10's reach narrows, deliberately.** A secret in a git-ignored file is no
longer reported. This is the trade the first declaration makes, and it is
consistent rather than new: `.env` has always been invisible to the sweep under
the dot-directory rule, and the policy in `conventions.md` is about what this
repository contains. A secret git refuses to store is not in the repository. The
second declaration is deliberately built not to make this trade a second time —
carried content stays swept precisely because it is committed.

**Silence is now possible where noise was.** A path declared carried stops being
linted, and a `.gitignore` line can make an artifact invisible by accident — a
domain that ignored `docs/` and kept a subdomain under it would lose coverage
without a violation to show for it. Reporting is the whole answer to this: all
three lists are printed and serialized on every lint, and `trellis tree` shows
the census beside them, so a steward reading a clean report can see what the
report did not cover. That is a weaker guarantee than a lint check and it is the
honest one — the kernel cannot know that an excluded path *should* have been an
artifact.

Nothing normative moves. `spec/model.md` is untouched and the spec stays v16:
which files a tool considers is a binding concern, and putting an exclusion
mechanism in the normative core would oblige every future binding to implement
this one. `checks/conventions-lint.md` gains a scope paragraph because it is the
single source both the kernel and the hand-walking steward mirror, and a steward
working without the binary needs the same boundary the binary has.

## Alternatives rejected
**A hardcoded deny list** — `node_modules`, `target`, `dist`, `vendor`,
`.venv`, `__pycache__`. Cheap and immediately wrong: it puts ecosystem knowledge
in a kernel that has none, is incomplete the day it ships, and is a rule nobody
declared and nobody can override for their own root.

**Read submodules from `.gitmodules`, or from `git ls-files --stage`'s 160000
gitlinks** — the precise answer to "what did this repo register as a submodule",
and a narrower question than the one that matters. A stray clone somebody dropped
in the tree is not registered anywhere and is just as alien; a submodule listed in
`.gitmodules` but never initialized has no files to skip. The `.git`-on-disk test
answers the question actually being asked — *is this directory another
repository's worktree* — for less work and one fewer git invocation.

**Infer artifacts from the model's shape** — treat only the locations
`spec/model.md` names as artifact-bearing, so anything under a deployment unit
is code by construction. Principled, and the failure mode is fatal: an artifact
placed somewhere the rule did not anticipate stops being linted *silently*.
Over-reporting is an annoyance; under-reporting with no signal is a governance
hole. Exclusion must be something the domain says, so that saying it is a diff
someone reviewed.

**A `.trellisignore` file** — familiar, and it puts the domain's boundary in a
dotfile nobody reviews rather than in the artifact whose whole job is declaring
this domain's rules. It also needs a glob engine; the registry needs prefixes.

**`.gitignore` alone** — solves the loudest symptom completely and leaves both
of the others: committed foreign markdown firing item 1 forever, and submodules
untouchable in principle rather than merely unhandled.

**The registry alone** — fully deterministic and free of git coupling, but it
makes every root hand-write the dependency directories of every language it
uses, and re-write them as dependencies change. The repository already states
that; asking it to be stated twice is how the two drift.
