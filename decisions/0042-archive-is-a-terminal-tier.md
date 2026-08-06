---
provenance: authored
status: accepted
date: 2026-08-05
---
# 0042 — Archive is a terminal tier, not a grouping

## Context
A root accumulates work that is over. `plans/` is the sharp case: it is the only
kind that grows without bound, has a terminal state, and has no intrinsic
ordering pushing dead entries away — `decisions/NNNN-` self-sorts by age, while
plan slugs are semantic by mandate (rule 10, and `/trellis:plan` says "never a
date"). The generated board and `trellis plan list` answer *what is live?* as a
reading, and that is the right answer to a different question: a reading does not
shrink the tree a human scrolls or an agent globs.

The model could not express the problem for most kinds. Only two carried a
terminal state — `retired` for a plan, `discarded` for a strategy. `problem/`,
`solution/{bc}/`, and `org/{role}/` carried no status at all, which is why the
spec prescribed "re-parent or archive" in five places and defined archive
nowhere. Nothing could be declared finished, so nothing could be cleared.

Rule 7 is what made this hard rather than obvious. A directory keyed on `status:`
is the textbook grouping it forbids, and 0026 had already refused `plans/{plan}/`
on the neighbouring ground that a sub-plan is the same kind. But 0026 answered
"same kind"; it never answered "same kind, finished".

## Decision
Rule 7 gains the line the refusals were always drawing: **a path may encode only
what is fixed for an artifact's life** — its kind, its provenance — and never a
state it can leave. `metrics/actuals/`, `solution/{bc}/`, and
`org/{role}/holder/` already satisfy this; a status directory would not, because
every transition would become a move and every ref a forwarding address.

Rule 13 then takes the one transition that never reverses. `archive/` holds the
terminal tier: this same hierarchy at `archive/{live path}`. Admission is
terminal-only — a kind with its own lifecycle uses its terminal value, a kind
without one gains `status: archived`, and a unit that archives as a subtree marks
its entry artifact. The live path stays the address, so refs resolve across the
move and append-only decisions (rule 6) never rot. Filing is a separate act from
finishing, on a retention horizon declared in `conventions.md` and executed by
the steward's archive sweep — the verdict is recorded in place, the move follows.
Archived artifacts stay governed (lint, refs, census) and leave attention
(dispatch, review gates, the effectiveness walk). `decisions/` is never filed.

That a tier is not a grouping is the whole distinction: `archive/` invents no
axis to browse by. It mirrors the live hierarchy, and nobody browses it — you
follow a ref into it, or you never look.

## Consequences
Spec v16 → v17: a hierarchy entry, a rule 7 clause, rule 13, and an optional
`status: archived` on `problem/{subdomain}.md`. The kernel gains
`archive/`-aware classification, address-stable ref resolution, `trellis archive`
with a `--sweep`, and lint item 25 (the tier mirrors the hierarchy, admits only
terminal artifacts, holds no decisions, and is never named by a ref).

One kernel change is load-bearing and was the reason to check before deciding.
`gitio::commits` ran `git log` with no `--follow`, so a rename truncated the
status timeline at the move: `views.rs` would see a one-entry timeline, `closed`
would never increment, and the board's closure rate and cycle time would read
zero and `—` forever. Every history-derived reading — the flow the Execution
health pattern specifies as "the history of `plans/`" — depends on following the
rename, and `--follow` cannot be paired with `--reverse`, which silently
truncates the same walk. The tests assert the readings are *identical* before and
after a move, which is the property the design exists to preserve; a test that
only checked the file moved would have missed exactly this.

The tier is not scaffolded. An empty `archive/` in a new root is the clutter this
decision is about; the first sweep creates it.

## Alternatives rejected
**Per-kind folders (`plans/_archive/`, `_plans/`)** — N conventions instead of
one, plans-only in practice, and a carve-out in lint item 11 that would weaken
0026's sub-plan ban. A root tier needs no carve-out: `subdirs("plans")` keeps
firing exactly as it did.

**A folder keyed on any non-terminal state** — `status: draft` already means
"nothing reviews it": dispatch skips drafts and the effectiveness checklist says
in terms that nothing evaluates one. A `_plans/` for drafts would duplicate a
state that already exists and put a move on `draft → ready`, the most frequent
transition in a plan's life and the one `trellis plan release` performs. Drafts
are the front of the queue, not clutter.

**Purge (`git rm` on a retention sweep)** — genuinely simpler: no hierarchy
change, no rule 7 clause, no rename-following, and history keeps everything. It
loses the artifact from the working tree, and a retired plan's verdict section is
operational memory an agent should be able to grep; `git log -S` is a real
retrieval path but not one anything reaches for unprompted. Reconsider if the
tier itself grows into the problem it solved.

**Leaving it to the board and `trellis plan list`** — the readings are right and
stay the recommended entry point (`AGENTS.md` now says so). They answer *what is
live?*; they do not shrink what a traverser walks, which was the requirement.

**The carried-content registry (0040)** — would silence the checks over an
archive directory, but it declares "the domain does not author this", which is
false of a retired plan, and it drops the artifacts out of discovery entirely.
Archived artifacts stay governed on purpose.
