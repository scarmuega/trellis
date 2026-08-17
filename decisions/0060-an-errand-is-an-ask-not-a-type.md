---
provenance: authored
status: accepted
date: 2026-08-17
---
# 0060 — An errand is an ask, not a type

## Context

0048 gave the operator a way to point at a plan and have its owner do
something to it without opening a session by hand. 0051 generalized the
mechanism: `[prompts]` became a name → template map, `refine` shipped as the
one requestable errand, and any additional key an instance declared was a new
errand with no recompile. The board read `GET /api/errands` and offered the
names.

Operating it showed the generalization went the wrong way. Every use of an
errand starts with an operator typing what they want done *now*, about this
plan, in this situation. That instruction is the errand. The name in front of
it adds nothing: the board's menu grew three hardcoded "refine" presets whose
only job was to paraphrase an instruction the operator was about to write
anyway, and the config-defined names were a pre-declaration of asks nobody
can pre-declare. Santiago's words: *all errands are, by definition, defined
in-situ by the user.*

Two smaller failures fall out of the same root. The board's control was
labelled **Act** but could never fire `act` — that name belongs to the
dispatch loop's own trigger, so the button named after the primitive could do
everything except it. And the one thing that genuinely does vary per
invocation — how much model and thinking effort this particular ask is worth
— had nowhere to go: sizing came from the plan's `complexity:` tier, which
describes the *plan*, not the errand somebody just thought of.

## Decision

**There is one errand and it has no name.** `[prompts]` keeps `act` and
`ritual`, the two prompts the runtime fires on its own, and loses `refine`.
There is no errand table, no `GET /api/errands`, no instance-declared errand
names. `POST /api/plans/{slug}/errand` carries `{instruction, model?,
effort?}` and that is the whole vocabulary.

**The framing is framework-authored and not configurable.** Something must
still say who to act as, that the runtime already resolved the mandate and
stamped the marker, where escalations go, whether this is delegated execution
(0057), and what the act procedure is — the operator's instruction cannot
carry that. `config::errand_prompt()` supplies it, in the binary, reachable
from no config key. It states no contract of its own: `refine`'s template
bounded the write target to the plan artifact, which was right for refinement
and wrong as the default for an ask nobody had written yet. What bounds an
errand session is what bounds every session — the mandate's `scope:` and
`authority:`, and the touched artifact's automation class.

**Sizing is chosen at the call.** `model` and `effort` ride the request and
win over the plan's tier; absent, the tier resolves as before. Both stay free
strings, as everywhere else the harness takes them — the kernel has never
held a model vocabulary and one invented here would date faster than the
models do. `/api/status` grows `session_tiers` so a client can offer the
domain's own configured values as the default rather than baking a model list
into itself.

**The request reports its own fate.** `request_errand` already replaced a
queued ask for the same plan; it now returns whether it did, and the route
answers `outcome: "replaced"` instead of `"requested"` — an instruction
silently displaced is an instruction lost. The response also carries
`budget_enforced`, false under the herdr backend where `--max-budget-usd`
cannot be honored (0043), because a ceiling with nothing holding it is worse
than no number at all.

This narrows 0051 rather than reversing it. Its load-bearing claim stands:
`act` is the sole role-invocation primitive, every session the runtime spawns
is `act` under some framing, and the binary carries one procedure. What goes
is the middle layer — that a framing must be *named in config before it can
be asked for*. Rituals keep their table, because a ritual genuinely is a
declared recurring thing; an errand never was.

## Consequences

- **Breaking for any root that declared its own `[prompts]` errand keys.**
  They stop being requestable, with no migration but "type the instruction".
  Acceptable because the feature is ten days old, undocumented outside the
  spec paragraph, and `~/Brain/txpipe` — the only real root — declares none.
- `trellis dispatch request <plan> "<instruction>" [--model M] [--effort E]`
  loses its errand positional; `trellis dispatch refine` is gone.
- `commands/refine.md` stays. It is the interactive plane's slash command,
  resolving a role and delegating to act — a procedure a human runs, not a
  type the daemon offers.
- `InFlightView` gains the herdr `pane`, which the pool always had and never
  surfaced: the board can now point at the session it started.
- Spec v23: `spec/runtime.md`'s operator-errand paragraph described the table
  normatively, so it moves with the code (release 0.23.0, decision 0056).
