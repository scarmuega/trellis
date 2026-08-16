---
provenance: authored
status: accepted
date: 2026-08-16
---
# 0058 — One source, two renderings

## Context

Decision 0050 made the spawn prompt a rendering of `commands/act.md` so a
dispatched session needs no installed plugin, and decisions 0045 and 0055
established the discipline that pays for itself: facts the runtime already
computed ride the prompt instead of being re-derived by the session. The
first dogfooding audit of that machinery — all fifteen dispatch-triggered
sessions on the txpipe root, 2026-08-13 through 08-16 — measured what the
discipline bought and what it still leaves on the table. It works: zero CLI
misuse, every session ended in a clean verdict, and each embedded directive
retired an observed misbehavior (the "never claim it again" clause ended the
re-claiming seen before it existed). What remains is friction, not failure:
every session opens with three to seven tool calls re-reading
`org/{role}/mandate.md` and `holder/system.md` — files the runtime read
moments earlier to compute the very facts in the prompt — and carries a
procedure of which roughly a third is dead under dispatch: `$ARGUMENTS`
parsing, the no-input report, interactive acting-role-marker mechanics that
the preamble then overrides with "leave it alone". The session pays a
reconciliation tax — preamble says X, procedure says Y-unless-dispatched —
for text that can never apply to it.

## Decision

**`commands/act.md` stays the single canonical source, and gains a second
rendering.** Spans that exist only for the interactive plane are fenced with
`<!-- interactive-only -->` … `<!-- /interactive-only -->` markers —
invisible comments to the harness's command loader, structure to the
runtime. `procedure::load()` strips fenced spans from every dispatch
rendering, after the frontmatter and with the same posture as its other
refusals: unbalanced markers are a startup error, never a silent half-strip.
Only what is dead under *every* dispatch flavor is fenced — the
delegated-execution exception (0057) and the human-holder branch stay,
because errands share this procedure and reach both.

**The mandate and the holder package become computed values.** Two new
placeholders in 0055's shape — self-contained, empty-safe, deterministic:
`{mandate}` renders `org/{role}/mandate.md` whole under a heading naming its
path, `{holder}` renders `holder/system.md` the same way when the holder is
a local package. A ref holder renders the empty string — a plugin agent
still adopts inline per the procedure, and a declared human never reaches a
runtime-triggered spawn (0045's short-circuit). The framing sentence inside
each value says what the preamble's other facts say: the read the procedure
asks for is already done, and the file is authoritative if they have
diverged. The runtime sets both variables at every spawn site; the default
`act` and `ritual` templates name them, and `refine` does not — the mandate
and holder are execution context, and refinement must not execute (0048,
the same line 0055 drew for `{skills}`).

## Consequences

- A dispatched session's startup read of the org tree collapses to the plan
  and `domain.md`; the mandate and holder arrive pre-read, the way
  `{escalate_to}` and `{automation}` already did.
- The dispatch rendering loses its dead branches and its reconciliation tax;
  the interactive `/trellis:act` file is byte-for-byte what it was, markers
  aside.
- An old binary refuses a new template naming `{mandate}` or `{holder}` at
  startup (`tmpl::check`'s gate) — 0055's designed failure, upgrade the
  binary before the template. A template that never names them renders
  exactly as before.
- The prompt grows by the two files it inlines — tokens the session was
  already paying for as tool-call round trips, now paid once at render time
  with no latency.
- Amended in the same commit (the 0041 precedent): `spec/runtime.md` (act
  row, dispatch facts), `template/runtime.toml` (placeholder list and prompt
  copies re-synced — now held to the embedded defaults by a lockstep test),
  `commands/act.md` (markers and the extended dispatch-preamble sentence),
  and `CHANGELOG.md`.
