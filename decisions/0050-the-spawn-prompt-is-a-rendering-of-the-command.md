---
provenance: authored
status: accepted
date: 2026-08-12
---
# 0050 — The spawn prompt is a rendering of the command

## Context

Every session the runtime spawns opened with a slash command — `/trellis:act
…`, `/trellis:refine …`, `/trellis:ritual …` — a token only the harness can
resolve, and only where the trellis plugin is installed in it. The premise was
stated three times over ("the daemon runs where the plugin is installed":
spec/runtime.md's act row, template/runtime.toml, the harness config's doc
comment) and it quietly hollowed out 0038's own claim. "Sessions are argv
templates in a committed runtime.toml … not a hardcode; a pi, Codex, or
OpenCode binding is a config edit" — except the first token of the default
prompt was a Claude-Code-plugin lookup, so the config edit bought a harness
that would be handed a command it cannot resolve. The argv was portable; the
prompt was not.

Meanwhile 0045 had already moved most of what the command does into the
runtime: root binding, the acting-role marker, the automation-class
derivation, `escalate-to:` resolution, the declared-human short-circuit, the
mechanical readiness share. What `/trellis:act` still contributes under
dispatch is prose — the conventions read, mandate binding, holder adoption,
provenance and escalation discipline, the report contract. Prose can ride a
prompt. The question was only where that prose may live, because 0048 had
just refused it one home: "the contract lives at the framework level, not in
a runtime prompt string."

## Decision

The commands stay canonical; the daemon renders them. `commands/act.md`,
`commands/refine.md`, and `commands/ritual.md` remain the single source of
each procedure — lightly rewritten to read invocation-agnostically (an
"Inputs" paragraph names the abstract inputs; the slash spelling and the
dispatch preamble are two mappings onto them). The binary embeds the three
files at build time (`include_str!`, the same seam that embeds `template/`),
and the default prompts become: a per-session-unique first line, the computed
facts 0045 already put there, and a `{procedure}` placeholder carrying the
frontmatter-stripped command body — refine and ritual rendered together with
the act procedure they delegate to.

This answers 0048 rather than overruling it: the contract still lives at the
framework level, in the same file the interactive plane reads; the prompt
string carries a *rendering* of it, produced from that file at the binary's
build SHA. No second copy of the discipline exists to drift — the sentence
0048 defends ("not in a runtime prompt string") was aimed at instance-owned
config prose, and the instance's `[prompts]` stays as thin as before: name
`{procedure}` or don't.

The seam moves and is renamed honestly. 0038 located harness swappability in
"a second `commands/` dialect"; for the spawn path that is now wrong — a
second harness needs no dialect at all, because the prompt arrives
self-contained. `commands/` remains the adapter surface for the interactive
plane (0019), and `--plugin-root` / `CLAUDE_PLUGIN_ROOT` swaps a live
checkout in place of the embedded copies at daemon startup — refused when
configured and unreadable, the same posture as `{plugin_dir}`.

Instances may put the slash spelling back. `[prompts] act = "/trellis:act
{owner} advance …"` renders exactly as before — the daemon always sets every
variable and the template chooses which to name. Degradation, not refusal.

## Consequences

- The installed-plugin premise is retired for the spawn path only. The
  interactive plane, the provenance-gate hook (`hooks/gate.mjs`), and the
  skills still assume the plugin where humans work — deliberately out of
  scope here.
- Binary-vs-installed-plugin skew becomes the operative drift: a domain
  pinning its plugin SHA (the spec-pin discipline) should run a binary built
  from the same checkout. This is the class of drift the spec pin already
  manages, not a new one — and no embedded-copy lockstep test is owed,
  because `include_str!` makes the copy identical by construction; what the
  tests guard instead are the sentinels composition depends on.
- `commands/**` joins the CLI's CI trigger paths: editing a command now
  changes the binary.
- The herdr prompt-echo needle contract becomes "the first line
  discriminates": every default prompt opens with the plan or ritual it
  concerns, and the needle cuts at the first line.
- spec/runtime.md's act row, template/runtime.toml's premise comment, and
  the harness config doc comment are amended in the same commit as the
  mechanism (the 0041 precedent).
