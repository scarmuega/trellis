---
provenance: authored
status: accepted
date: 2026-08-04
---
# 0039 — Retire the forge clock; the daemon is the runtime

## Context
0038 landed `trellis serve` and left both clocks shipping, with
`conventions.md` making each instance pick one. That was the cautious move and
it was wrong on the evidence: the workflows were never really used. They are
three files a scaffolded domain inherits, must read, must choose against, and
must then delete — plus a kernel subcommand (`trellis rituals cron`) whose only
job was checking that a cron block agreed with `rituals.md`, plus a paragraph
in `conventions.md` explaining which clock this instance runs and why two would
run every ritual twice.

That is the cost of an alternative nobody exercises. Worse, it is the expensive
kind: an unused path that still has to be described accurately in the spec, the
template, and the instance's own conventions, and that goes stale silently
because no one runs it. The framework's own argument against carrying
unexercised structure applies to the framework (premise 5 — beliefs are
revisable, and this one was held for eleven days).

## Decision
**The GitHub Actions binding is removed.** `template/.github/workflows/` —
`dispatch.yml`, `rituals.yml`, `ingress.yml` — is deleted, and with it
`cli/src/rituals.rs` and the `trellis rituals cron` subcommand. `trellis serve`
is not "the local binding" alongside a forge one; it is **the** binding, and
`spec/runtime.md` now describes one.

**The forge keeps review and loses triggering.** PRs remain the approval gate,
`.github/CODEOWNERS` is still generated from `owner:` frontmatter, branch
protection still makes core-class human review a property of the forge rather
than of agent obedience, and `github:` stays on `holder/ref.md`. None of that
is Actions; all of it is the automation policy in `spec/model.md` reaching the
mechanics that enforce it. What goes is the forge as a *trigger*: nothing
schedules, dispatches, or invokes a role from it.

**`ingress` becomes unbound, and says so.** It was bound only by
`ingress.yml`, and 0038 had already deferred it there. Removing that workflow
leaves the event-driven plane with no binding at all, which is recorded as a
binding choice in both `spec/runtime.md` and the template's `conventions.md`
rather than left to be discovered. An outside event reaches the domain when a
human brings it into a session. The conforming-binding checklist gains a line
for exactly this: an absent plane is a choice to state.

The tempting shortcut is refused on purpose. The daemon already listens on a
socket, so an ingress endpoint is a few lines away — and it would be a trigger
plane with no mandate behind it, which is the argument 0038 made for keeping
the serving surface read-only by construction. Ingress gets bound when a real
event needs it, by something that carries a mandate.

**Cron generation retires with the clock it served.** It was named in Stage 2's
charter and shipped in 0037, and the daemon reads `rituals.md` at every pass,
so there is no cron to emit and no drift to check. Keeping a subcommand whose
output targets a workflow file the template no longer ships would be a vestige
with a help string.

## Consequences
A scaffolded domain is smaller and has one way to be operated: review
`runtime.toml`, run `trellis serve`. No cron to keep in step with `rituals.md`,
no second clock to disable, no `role:` labels to create, and one fewer secret
in a forge. The spec describes one binding instead of two, which is the honest
count.

The costs land squarely on availability and reach, and are recorded in
`spec/runtime.md`'s known limits rather than softened. **The clock now runs
where the daemon runs**: a laptop that sleeps misses cadences, where a forge
cron ran on someone else's always-on machine. Catch-up on waking fires each
overdue row once, which bounds the damage without removing it, and a domain
that wants uptime runs the daemon somewhere with uptime. **There is no
event-driven plane at all**, so a domain whose work genuinely arrives as
outside events has to bring them in by hand until something binds ingress.
**The serving surface is single-machine**, so two operators over one repository
see two boards.

Nothing normative moves. The spec stays v16, `spec/model.md` is untouched, and
the three trigger planes remain the shape a runtime must have — this binding
simply leaves one of them empty, which the model always permitted and the
checklist now asks bindings to declare.

Decisions 0019, 0029, 0032, 0033, 0037, and 0038 all reference the workflows.
They are not edited: a decision is append-only, and a record that quietly
stopped describing what shipped would be worse than one that is plainly
historical. This decision supersedes their binding mechanics; their arguments —
dispatch as `schedule` + `act`, the complexity map living with whoever runs the
scan, holds as declared-field reads — survive intact, because those were never
claims about a forge.

## Alternatives rejected
**Keep the workflows as a documented alternative** — the status quo 0038 chose,
and the thing this decision reverses. An alternative that is never exercised is
not free: it is described in three places, chosen against in every scaffold, and
wrong before anyone notices.
**Keep `ingress.yml` alone** — it is the one workflow with a capability no other
binding replaces, which is exactly why keeping it is a trap: it preserves the
whole Actions dependency (secrets, plugin checkout, runner) for a plane no
domain here has used, to avoid writing down that the plane is unbound.
**Add ingress to the daemon now** — a socket is already open, so this is cheap
to build and expensive to be wrong about. It reverses 0038's read-only argument
one commit after making it, and an unauthenticated trigger door is a plane with
no mandate. Waits for a real event.
**Keep `trellis rituals cron` for hand-rolled schedulers** — a systemd timer or
launchd agent supervises `trellis serve`, which owns the cadences itself;
neither wants a cron block per ritual. The subcommand's `--emit` output is
GitHub-shaped, so keeping it would mean keeping a GitHub artifact generator in a
kernel with no GitHub binding.
**Drop the forge entirely, including PRs and CODEOWNERS** — a much larger
change wearing this one's clothes. The automation policy in `spec/model.md`
maps core-class change to a structurally required human review, and the forge is
what makes that structural rather than obeyed. Removing it needs its own
decision and a replacement gate, not a cleanup.
