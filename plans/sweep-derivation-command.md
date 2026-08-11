---
provenance: authored
owner: scarmuega
status: draft
type: initiative
complexity: standard
---
# `trellis sweep derivation` — the derivation sweep's mechanical share

## Context

The steward's derivation sweep is the most expensive ritual left fully manual:
an agent session does git archaeology (commits touching `market.md`,
`strategy/`, `contracts/` since the last sweep, plans retired in the window)
and then computes five downstream fan-outs by hand — induced subdomains,
affected plans, scoped mandates, orphaned subdomains, economic orphans,
released-but-abandoned `awaits:` targets, contract consumers per
`context-map.md`. Nearly all of that machinery already exists uncalled:
`graph.rs` computes orphans and funding edges, lint items 3/14/17/19/20/21
encode the classifications, `gitio` reads history. This is the same shape 0045
extracted for readiness: verdict machinery with no caller.

## Objective

One command — `trellis sweep derivation --since <date|ref>` (default: the last
sweep's `state.json` entry when run by the daemon, `--since` required
interactively) — that emits the sweep's mechanical findings as
escalation-shaped rows (`artifact`, `owner`, `trigger`, `why`), JSON and text,
so the steward session walks only the judgment reads: whether a contract edit
semantically breaks a consumer, whether a retired plan's verdict reads
abandoned, what a re-parenting should look like.

## Approach

- Change detection: `gitio` diff of the window for `market.md`, `strategy/**`,
  `solution/*/contracts/**`; plans whose `status:` flipped to `retired` in the
  window (the existing status-history reader).
- Fan-out, per change kind: edited/discarded strategy → its induced subdomains
  (`Derived`), plans whose `subdomains:` touch them, mandates whose `scope:`
  names them; edited contract → consumer contexts from `context-map.md` rows
  naming the publishing context; retired plan → `awaits:` dependents.
- Standing classifications (orphans, economic orphans, incomplete pivots) stay
  lint's; the command may re-emit them under a `standing` key for one-stop
  reading, computed by the same functions — never duplicated logic.
- Daemon wiring: none required to start — the steward ritual session calls the
  command. A later step could have the daemon pass `--since` from `state.json`
  the way dispatch carries its facts.
- `agents/steward.md`'s derivation-sweep section shrinks to: run the command,
  judge its rows, report per owner.

## Measurement

A derivation-sweep session on a real domain (txpipe) spends its turns on the
judgment reads, not on git archaeology: compare session turn/token counts
before and after, same domain, same window.

## Risks and escalation triggers

The context-map consumer read depends on `context-map.md` table hygiene (lint
item 23); rows the lint flags should surface as warnings, not silent drops.
