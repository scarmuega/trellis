---
provenance: authored
status: accepted
date: 2026-08-11
---
# 0046 — Dispatch is a loop, rituals are a clock, serve is a window

## Context

`trellis serve` was one process owning three unlike things: a continuous
dispatcher, a wall-clock scheduler, and a read-only serving surface. The
seam showed operationally the same day 0045 landed: an `awaits:` chain
released its next plan minutes after the daily dispatch pass had fired, and
the plan sat dispatchable for up to a day — the operator watching a ready,
unheld plan that nothing would touch. The daily cadence on dispatch was
never semantic: it is a fossil of the forge cron the scan was ported from
(0037), kept through 0039 because the daemon inherited the shape. A
ritual's gap, by contrast, *is* semantic — the metric sweep's cadence
defines the freshness window, the archive sweep's rhythm is the retention
policy — and every correctness property of dispatch (claims flip plans out,
holds self-clear, the scan reads declared fields) holds per-tick exactly as
it held per-day.

## Decision

Three processes, split by what their time means. Pure split: each does one
thing, none requires another.

- **`trellis dispatch run`** — the pull loop. Every tick it polls the tree
  and spawns one act per dispatchable plan; no calendar gate at all. The
  `plan dispatch` row in `rituals.md` stays as the operator-of-record
  declaration and the scheduler skips it. What the daily cadence provided by
  accident the loop provides on purpose: a **retry cooldown**
  (`scheduler.retry_cooldown_secs`, default 900) skips a plan whose session
  ended without claiming, or a crash-looping act would respawn every tick at
  session prices. Reporting is by transition — a hold announced when it
  appears and when it clears — never once per tick. The dispatcher hosts the
  sessions' MCP back-channel on its own loopback socket (`dispatch.addr`):
  the channel belongs to the process that owns the session lifecycle, and
  serve was carrying it incidentally.
- **`trellis rituals`** — one idempotent pass of the wall-clock work: fire
  what the cadences owe today, drain the sessions, record, exit. The OS is
  the wall clock — cron, launchd, or a hand may invoke it hourly and it
  fires only what is due. `catchup` keeps its meaning for windows missed
  while nothing was running. It opens its own ephemeral channel
  (`rituals.addr`) for the sessions it drains.
- **`trellis serve`** — the read-only window: tree, views, board, and the
  dispatcher's `status.json` snapshot overlaid on `/api/status`, labelled
  with whether that process is alive. No clock, no spawning, no writes; may
  be down without stopping anything.

Split state (`dispatch-state.json` / `rituals-state.json` — two writers on
one document is a race; a legacy `state.json` is read once, filtered by
keyspace), split concurrency caps (a ritual batch cannot starve dispatch),
prefix-filtered herdr adoption (each process adopts only its own orphans),
and a lockfile on the acting-role marker (now genuinely two writers).
`trellis inbox` resolves the dispatcher's address first.

## Consequences

- An `awaits:` release, a fresh `plan release`, or an unblock dispatches
  within one tick — a chained program flows through in session-time, not a
  day per link.
- The operator runs two things instead of one (a dispatcher process and a
  cron line), plus the optional viewer. That is the cost of the pure split,
  taken knowingly over a composed mode: three shapes in one process is how
  the fossil cadence survived six decisions.
- `serve --once`, `--no-http`, `--tick-secs`, `--map`, and `--dry-run` are
  gone from `serve`; their homes are `dispatch run` and `rituals`.
  `scheduler.dispatch_cadence` is parsed, ignored, and noted as obsolete.
- The spec's "the daemon is the domain's only clock" framing is retired:
  rituals borrow the OS's clock, dispatch is a loop rather than a clock,
  serve carries none.
- A machine that sleeps now misses nothing by itself: the next `rituals`
  invocation settles the calendar, and the dispatcher reacts to the tree
  whenever it runs.
