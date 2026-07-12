---
provenance: authored
status: accepted
date: 2026-07-12
---
# 0019 — Runtime: a contract with harness bindings, extracted by pull

## Context
The spec defines the operating model but not the runtime that executes it —
deliberately: rule 2 leaves ref resolution to "the consumer's tooling." Operating
a business day-to-day needs three trigger planes: interactive sessions at the
domain root; rituals executed on cadence with human-in-the-loop channels; and
external events (a support ticket, an inbound email) invoking mandated roles.
Candidate shapes were each proposed as *the* runtime — plugins for existing
harnesses, a custom harness (pi- or SDK-built), a hosted service — and each
carries lock-in, audience fragmentation, or premature infrastructure.

## Decision
No harness is the runtime. The runtime is a **contract plus bindings**, mirroring
the mandate/holder seam (rule 4: identity is portable, authority is not): the
domain repo stays harness-neutral, `spec/runtime.md` — a non-normative companion —
defines the services any conforming runtime provides (**session, act, schedule,
ingress, gate, escalation, ledger**), and each instance declares its binding in
its `conventions.md`. All three planes reduce to one primitive, `act(role,
input)`: resolve the local mandate, bind its holder, execute within authority,
escalate beyond it.

The **reference binding is Claude Code plus the forge**. The plugin supplies
session (existing), act (`/trellis:act`), rituals (`/trellis:ritual`), and a
deterministic gate (`hooks/`); the forge supplies scheduling (Actions cron),
ingress (issues labeled `role:{name}`), HITL (escalations are issues, approvals
are PRs, core-class review via branch protection and generated CODEOWNERS), and
the ledger (git history plus forge threads). The template ships reference
workflows.

Further extraction happens by pull, per premise 5. **Stage 2** — the CLI already
chartered in the steward mandate — lands when dogfooding shows which checks must
never be probabilistic: deterministic lint, `mandate.md` → permission-profile
compilation, `rituals.md` → cron generation, CODEOWNERS generation. The CLI is
the harness-neutral kernel that makes second bindings (pi, Codex, OpenCode)
cheap. **Stage 3** — a thin always-on dispatcher on the Claude Agent SDK — lands
only when an ingress event cannot be relayed through the forge or needs
conversational latency.

## Consequences
Lock-in confines to adapter surfaces (`commands/`, `hooks/`, workflow files);
fragmentation is answered by the contract rather than by ports. Accepted v1
limits, recorded in `spec/runtime.md`: mandate authority is enforced by prompt
plus a deterministic backstop, not compiled permissions (stage-2 territory); the
gate does not see shell-mediated writes; acting-role attribution rides on an
ephemeral session marker (`.trellis/acting-role`).

## Alternatives rejected
A single-harness plugin as the end state — makes today's convenience structural
lock-in and leaves non-Claude audiences unserved. A custom pi- or SDK-built
harness first — rebuilds session, skills, permissions, and sub-agents that
existing harnesses give free, before any usage-hours justify owning them. A
hosted cloud runtime — infrastructure ahead of evidence inverts premise 5;
nothing about zero-usage-hours operation demands an always-on service the forge
cannot already provide.
