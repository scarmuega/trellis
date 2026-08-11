---
provenance: authored
status: accepted
date: 2026-08-11
---
# 0045 — The act ceremony's deterministic share moves into the runtime

## Context

A measured dispatched act (a `standard`-tier plan, opus/xhigh) spent ~10–15%
of its tokens and ~25 of its turns on ceremony that is deterministic: binding
the root the daemon was already serving, re-resolving a mandate whose
frontmatter the scan had already parsed, creating `.trellis/acting-role` (a
dotfile write that also blocked an unattended session on a permission dialog),
re-verifying readiness items the CLI can compute, hand-editing status flips
the CLI already has verbs for, and a Task delegation to the holder agent that
re-read ~40KB of mandate, conventions, and plan into a second context. Worse
than the overhead were the sessions that should never have spawned: a
mechanically unready plan bought a full session whose entire work product was
discovering the unreadiness, and a human-held owner bought one that read
everything and correctly stopped at act's never-impersonate rule.

The steward mandate names the cure for its own jobs: "extract the
deterministic parts into tooling." Most of the tooling already existed —
`readiness.rs` computes the mechanical share with an honest `mechanical_pass`
verdict, `trellis plan claim/block` and `trellis escalate add` encode the flip
and record ceremony — and was simply not wired into the runtime or named in
the plugin's instructions.

## Decision

The runtime performs every deterministic act-ceremony step; sessions keep the
judgment.

1. **Dispatch prechecks readiness — and holds, never blocks.** The scan runs
   the mechanical readiness share per `ready` plan and holds failures the way
   it holds `awaits:` — skipped, still `ready`, nothing written, the failed
   items named on the scan surfaces. The block flip and the escalation record
   stay a mandated session's acts (`runtime.md` reserves both for owners);
   the alternative — the precheck acting *as* `org/steward` via
   `trellis plan block` — was considered and declined: mechanical unreadiness
   is self-clearing (fix the ref, the next scan releases), and `blocked`
   means a defect a human must clear.
2. **A declared-human holder gets a handoff, not a session.**
   `holder/ref.md` gains an optional `kind: agent | human` (spec v19) — the
   machine-readable form of act's prose-only split. `human` short-circuits
   dispatch into a reported handoff; absent keeps pre-v19 routing, so nothing
   breaks. Lint warns (never fails) when a role owning ready plans leaves it
   undeclared.
3. **The daemon owns `.trellis/acting-role` for the sessions it spawns** —
   one line per live session (`role date key`), stamped at spawn, removed at
   retirement, reconciled at startup so a crash cannot leave a stale line and
   an adopted session keeps its own. The gate tests existence and needed no
   change; interactive invocations still write the act.md two-line form, and
   the daemon never touches lines that are not its own.
4. **The prompt carries computed facts and names the CLI verbs.** A new
   `{escalate_to}` placeholder (owner mandate's `escalate-to:`, falling back
   to the owner), plus prompt text stating the precheck already ran and
   pointing at `trellis plan claim` / `trellis plan block --asks` /
   `trellis escalate add` instead of hand-edited frontmatter and hand-written
   YAML.
5. **The holder branch of act adopts plugin agents inline.** Task delegation
   duplicated the whole trellis context; act.md now instructs the session to
   read the agent definition and continue as it, mandate already bound.

## Consequences

- The wrapper cost of a dispatched act drops by roughly its measured share
  (~10–15% of tokens, ~25 turns), and the permission dialog that blocked
  unattended acts on marker creation is gone.
- Sessions are no longer spent on mechanically unready or declared-human
  plans; both surface as scan output (log, board, `/api/dispatch`) instead.
- The scan reads plan bodies now (the deferral-token scan), not only declared
  fields — still deterministic, but a wider read than v18's "declared fields
  only" phrasing, which `runtime.md` now states.
- A readiness hold is quiet by design: it re-enters the scan every tick and
  toasts nothing (toasts remain escalation-record-only). The board and the
  scan log carry the reason; a domain wanting louder failure keeps the option
  of releasing plans only through `trellis plan release`, which runs the same
  gate interactively.
- Spec bumped to v19 (`kind:` schema, precheck and handoff in dispatch).
