---
provenance: authored
status: accepted
date: 2026-08-16
---
# 0059 — The mandated hand-off is a verb

## Context

Release 0.22.1 named the mandated hand-off a verdict of its own: a session
whose mandate prescribes a relay ends by giving the plan a new `owner:` and
`status: ready`. But the verdict clause described that move abstractly ("via
the trellis CLI — the mandate's spelling wins") because there was nothing
concrete to name: `trellis plan handoff` parks an active plan on a PR ref,
and no verb performs the owner flip. The txpipe transcript audit showed what
abstraction costs — eight of fifteen sessions improvised the relay as two or
three raw `trellis fm set` calls, each session spelling the sequence
slightly differently, none validating the target role, every one leaving a
window where the plan carries a new owner and a stale status. Every other
lifecycle transition is a guarded verb precisely so sessions cannot improvise
it; the transition the QA-relay mandates most often was the one exception.

## Decision

**`trellis plan pass <plan> --to <role>` is the relay hand-off.** One
guarded move: the target role must exist under `org/` (the bare name is
accepted and normalized to `org/<role>`), the plan must be `active` — the
session-verdict case — or `ready` — the operator reassigning a queued plan;
draft, blocked, and retired are refused like any illegal flip. The verb sets
`owner:`, flips an active plan to `ready`, and clears any stale `handoff:`
ref — a passed plan must not stay parked on the previous taker's proposal.
`pass` moves the plan to its next taker; `handoff` parks it under its
current owner. The names now say so.

**The verdict clause names every exit as a command.** The act template's
closing instruction spells all four verdicts verbatim with the plan path
pre-filled — retire, block, pass, park — because the audit's one repeatable
lesson is that a directive embedded as a concrete command is followed and a
directive described as an outcome is improvised.

Deliberately not decided here: board parity (`status_flip` in 0049's window
does not gain a `pass` route yet — the operator's surface can follow when
the need appears), and the retry cooldown, which still delays a passed
plan's re-dispatch by one bounded window exactly as it does after any
verdict.

## Consequences

- A relay mandate's hand-off is atomic, validated, and identical across
  sessions; the `fm set` improvisation and its stale-status window are gone.
- The scan needs no change: it already dispatches on `ready` + `owner:`, and
  `relinquish` sees a passed plan as `Untouched` — the verdict was the
  session's own.
- A ghost role is refused with the role list — the check act.md's stripped
  "list the roles and stop" sentence used to delegate to the session now
  lives where every other guard lives, in the CLI.
- Amended in the same commit (the 0041 precedent): `spec/runtime.md` (the
  verdict prose names the verb), `skills/conventions/SKILL.md` (lifecycle
  verbs), the default act template, `template/runtime.toml`, and
  `CHANGELOG.md`. No schema is added and the spec does not bump — the verb
  performs a transition `spec/runtime.md` already leaves alone as one the
  session moved itself (0055's precedent for the placeholders, 0.22.1's for
  the verdict clause) — so this ships as patch release 0.22.2.
