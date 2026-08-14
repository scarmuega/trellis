---
provenance: authored
status: accepted
date: 2026-08-14
---
# 0055 — Skills ride the spawn prompt as an index

## Context

Decision 0050 made the spawn prompt self-contained and named its own residue:
"the skills still assume the plugin where humans work — deliberately out of
scope here." So a dispatched session did the same work as an interactive one
with none of the domain's skills reachable — or worse, with whatever skill
inventory the operator's harness happened to have installed, dozens of
unrelated descriptions competing for attention while the two or three
procedures the plan actually touches sat unlisted in the tree. Skill
availability was the one input to a spawned session that was neither
deterministic nor harness-agnostic.

The facts needed to fix that were already computed. Skills have exactly two
homes (decision 0021): `solution/{bc}/skills/` for business procedure and
`org/{role}/holder/skills/` for identity-bound technique. A plan names its
contexts in `contexts:` and its role in `owner:`; a rituals.md row names its
executor. The dispatch scan reads all of them before every spawn. And the
harness mechanism being reproduced is smaller than it looks: native skill
support in any harness is (a) descriptions injected into context and (b) lazy
body loading on demand. (a) is prose, which rides any prompt; (b) needs only
a file read, the one capability every agent has. The substrate was chosen for
this in decision 0001.

## Decision

**A new `{skills}` placeholder renders a deterministic index, never bodies.**
At spawn the runtime resolves the session's skills — a plan session gets the
owner's holder skills then each `contexts:` home's, in declared order; a
ritual gets its executor's holder skills only — and renders one line per
skill: name, derived description, path. The session reads a listed file
through its own tools when the work calls for it. Availability and scope
become computed facts like `{escalate_to}` and `{automation}` (decision
0045); invocation remains the session's judgment, exactly as it is for a
harness-native skill list. Any harness that takes a prompt gets the whole
mechanism; there is no plugin dialect to port.

**The rendered value is self-contained and empty-safe.** The heading and the
standing instruction (read the file in full before doing covered work; the
skill is authoritative over your general approach) live inside the value, so
a template names `{skills}` on one line and owes no framing. No skills
renders the empty string — naming the placeholder costs nothing, and the
default `act` and `ritual` templates name it unconditionally.

**Name and description are derived, never declared.** The directory name is
the name (spec rule 10 — names are retrieval keys). The primary doc is
`README.md`, then `SKILL.md` (the spelling plugin-style skill repos import),
then the sole direct-child `.md`; a doc-less skill still lists by its
directory path. The description is the frontmatter `description:`'s first
line, else the first H1, else the first prose line — lenient the way every
frontmatter read is, because domain skills carry only the universal artifact
schema (`provenance:`, `owner:`, `tags:`) and no skill-file convention exists
to require. No schema is added and the spec does not bump.

**Default act and ritual name it; refine does not.** Refinement reshapes a
plan's content and never executes it (decision 0048) — handing it the
execution procedures would invite exactly the scope creep the refine contract
forbids. The runtime still sets the variable at every spawn site, so a custom
errand template may name `{skills}` and an instance that rewrites the refine
template owns that variant, as always.

Deliberately **not** decided here: suppressing the harness's own skill
inventory. A prompt can add context; it cannot reach into a harness's system
prompt and remove what the operator installed. For the Claude Code reference
binding the noise-suppression spelling is two argv lines an instance may add
to its `act_cmd` — `"--setting-sources", ""` and `"--settings",
"{\"disableBundledSkills\":true}"` — adapter configuration in the file that
already owns adapter configuration, never shipped as a default, and a
reminder that a session stripped this way loses settings-borne hooks too.

## Consequences

- The 0050 residue shrinks by one clause: skills now reach dispatched
  sessions through the prompt, and `--plugin-dir` remains only for the hooks
  and the interactive plane's invocable skills.
- An old binary refuses a new template naming `{skills}` at startup
  (`tmpl::check`'s placeholder gate) — the designed failure, not a spawn-time
  surprise: upgrade the binary before the template. The reverse direction is
  silent by construction: a new binary always sets the variable, and a
  template that never names it renders exactly as before.
- Index-not-bodies keeps the prompt cost flat in skill count; a skill's
  token cost is paid only when covered work appears, and the read lands on
  the tree's current state, not a spawn-time copy.
- The derivation rule is the first code to care what is inside a skill
  directory. It asks for nothing new — but a skill whose first line is worth
  reading in an index is now worth writing, which is rule 10 applied to
  descriptions.
- Amended in the same commit (the 0041 precedent): `spec/runtime.md` (act
  row, dispatch facts, session services row), `spec/patterns.md`
  (cross-cutting procedures), `template/runtime.toml` (placeholder list, the
  `{skills}` note, and its prompt copies re-synced to the embedded defaults
  they had drifted from), `commands/act.md` (the dispatch preamble may carry
  the index), and `CHANGELOG.md`.
