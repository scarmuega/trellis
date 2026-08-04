---
provenance: authored
status: accepted
date: 2026-08-04
---
# 0038 — Canonical local runtime: `trellis serve`

## Context
The reference binding (0019) claimed a runtime is swappable, and the harness
is: `act` is a command, and a second harness is a second `commands/` dialect.
The forge never was. Three of the seven contract services — `schedule`,
`ingress`, and the dispatch wiring that composes `schedule` + `act` — exist
only as GitHub Actions crons. A domain operated from a laptop against a bare
git remote therefore has an interactive plane and nothing else: no clock, no
dispatch, no scheduled rituals. That is not a binding choice an instance
records in `conventions.md`; it is a dependency the reference binding never
declared it had.

Two smaller pressures point the same way. 0036 accepted "an escalation record
notifies nobody" and chartered the fix — "a binding that needs a push adds one
*over* the records" — with no substrate to add it to: there is no process
running to notice a record open. And the five generated views are files. A
board is refreshed by regenerating a markdown table; nothing serves the tree,
so every UI over a Trellis domain starts by reimplementing the tree walk.

What changed on the cost side is 0037. Before the kernel, this daemon was a
build: root detection, frontmatter parsing, the scan, dwell arithmetic, view
generation. After it, all of that is a tested library in the same binary, and
the daemon is a timer, a process spawner, and a socket.

## Decision
**`trellis serve` — a daemon mode of the existing kernel binary, run at a
domain root.** One binary, because a second one re-forks root detection and
scan semantics into an artifact that can disagree with the first, which is the
drift 0037 exists to end.

- **It owns the clock.** `rituals.md` rows are read at every tick and fired on
  their cadence; the dispatch scan runs in-process on its own. Cadences are
  read, never compiled, so the "keep the cron in step with `rituals.md`" drift
  class does not exist for this binding — `rituals cron --check` remains the
  forge binding's tool. A cadence with no day count (`on demand`) is never
  fired automatically and is *reported* as unscheduled, so the silence is
  visible rather than assumed.
- **Sessions are argv templates in a committed `runtime.toml`.** `claude -p
  "/trellis:act …"` is the default, not a hardcode; a pi, Codex, or OpenCode
  binding is a config edit, and so is the fake harness the tests run against.
  Templates are argv arrays rather than shell strings: a placeholder fills one
  argument slot, so no artifact's content can become a shell word. The
  complexity→session map moves into that file for this binding — the map lives
  with whoever runs the scan, which leaves `dispatch.yml`'s `--map` flags
  exactly where 0032 put them.
- **It serves the tree read-only**: a JSON API over the same facts the
  commands print — `plan list`, `show`, `escalate list`, `dispatch scan`, the
  rendered views — plus an embedded static board page over that API. Read-only
  is structural, not a policy setting: there is no write path, so every change
  still enters through a session bound by its mandate, the gate, and the
  artifact's automation class. The same property is why the HTTP surface is
  **not ingress**: a door into `act` reachable without a mandate would be a
  fourth trigger plane wearing the second one's clothes.
- **It defines the escalation channel seam** and ships no adapter. A record
  that newly reads `open` is detected once and offered to registered
  transports; with none registered, the live channel is the pull surface. A
  transport configured but unimplemented is refused at startup rather than
  silently dropped.
- **It is never the interactive plane, and never judgment.** Serve starts
  sessions; roles act. Ingress stays with the forge relay.

**On 0029 and the staging entry.** 0029 rejected this daemon eleven days ago as
"infrastructure ahead of evidence," and `spec/runtime.md`'s Stage 3 promised
something narrower still: an ingress-only dispatcher on the Claude Agent SDK,
pulled by an event the forge could not relay, or by conversational latency.
Neither pull fired. Saying that plainly matters more than claiming the entry
was prescient: the staging discipline is only worth keeping if a stage can be
recorded as *wrongly predicted* rather than retrofitted. What actually pulled
is different on all three counts — local-first operation had no scheduled
plane at all, 0036 had chartered a push seam with nowhere to live, and 0037
turned the daemon from a build into a composition. Premise 5 is not violated by
this; it is the premise being applied. The belief that a forge cron sufficed
was held on the information available, the information changed, and the belief
is revised with the trail intact. What has *not* changed: the latency argument
0029 deferred on still has not fired. Responsive dispatch is a side effect of
owning the clock, not the reason to own it.

**Reference and local are two bindings, not a replacement.** The forge binding
stays the reference: it alone covers all seven contract services, because serve
defers ingress. Serve is the canonical *local* binding, and the expected shape
for a domain running both is that serve owns the clock while the forge keeps
ingress and approvals. An instance declares which clock it runs in
`conventions.md`; running two is the footgun, and the template says so in both
places.

## Consequences
A domain can be operated from a checkout, one binary, and a harness. The forge
demotes from structural to optional for planes 2 and 3, which makes 0019's
swappability claim true rather than aspirational, and gives the escalation push
seam a place to live when a domain pulls for one.

New limits, recorded in `spec/runtime.md` rather than left to be discovered: a
machine that sleeps misses cadences, mitigated by catch-up on waking from the
last-run state and honest about firing once rather than once per missed window;
the serving surface and the session logs are single-machine, so the durable
ledger is still git; the API is unauthenticated and bound to loopback, so
exposing it is a deliberate act; and a domain that enables serve while leaving
the forge crons on runs two clocks and will run its rituals twice.

Two costs accepted. The kernel gains a long-running mode and two dependencies
(a synchronous HTTP server and a TOML parser) — the price of one binary, paid
deliberately over a second one that drifts. And the complexity map now has two
tuning points, one per binding; the alternative couples the forge workflow to a
file it otherwise ignores, and the map belongs with whoever runs the scan.

A committed non-markdown file enters the root. `runtime.toml` sits beside the
workflow YAML the forge binding already commits there, and lint item 10's
secrets sweep already covers it, so premise 8 is strained no further than it
already was.

**No lockstep coverage, deliberately.** `cli/tests/lockstep.rs` guards second
encodings of normative prose — checklist items, closed enums, spec pins. Serve
encodes no rule: its inputs are cadences and plan frontmatter, read through
`registries` and `dispatch`, which lockstep already holds to the model. A serve
entry in that file would be coverage theater. The spec stays v16: this is
consumer tooling, `spec/model.md` is untouched, and `runtime.toml` is
binding-owned config on 0037's `generated-by:` precedent.

## Alternatives rejected
**Keep the forge crons as the only clock** — leaves the forge structural, the
swappability claim false, and 0036's chartered seam with nowhere to attach.
**A separate `trellisd` binary or workspace crate** — the clean-layering
instinct, and wrong here: it re-forks root detection, tree loading, and scan
semantics into a second artifact that can disagree with the first. The module
boundary inside `cli/src/daemon/` carries the same distinction at no
distribution cost, and the crate doc states the rule the layout implies.
**Embedding the Claude Agent SDK** — what Stage 3 named, and it hardcodes one
harness into a kernel whose whole argument is that a second binding should be
cheap. Command templates keep the harness an adapter.
**An async runtime (tokio + axum)** — the workload is a sleeping tick loop,
process spawns, and localhost requests dominated by blocking git shell-outs.
Async buys nothing here and costs dozens of transitive crates against a build
this repo keeps small.
**Caching the tree behind a filesystem watcher** — a stale board is worse than
a slow one at this scale, and a cache is a second source of truth for the
question the API exists to answer. Recomputing per request stays until a
measurement says otherwise.
**A write or ingress path on the HTTP surface** — the fastest way to a
convenient door with no mandate behind it. Every write goes through a session.
**Shipping the push transports now** — the seam ships, the transports wait.
Pull-only first is premise 5 applied inside the decision that relaxes it.
**A hosted cloud runtime** — still rejected, unchanged since 0019. Serve is
part of the argument for why it stays unnecessary.
