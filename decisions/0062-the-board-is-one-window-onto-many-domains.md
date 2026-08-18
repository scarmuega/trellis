---
provenance: authored
status: accepted
date: 2026-08-17
---
# 0062 — The board is one window onto many domains

## Context

A domain is a sovereign root (decision 0002) and each runs its own processes
against it (decision 0046): one `trellis serve`, one `trellis dispatch run`,
one port, hand-assigned in that root's `runtime.toml`. Eight roots means eight
ports, and that isolation is worth keeping — it is what lets a domain move to
its own host without any other domain noticing.

The board did not match. It was a single-domain window: every call a
root-relative `/api/...`, the origin chosen at dev-server startup by
`TRELLIS_API` and fixed for the life of the process. Switching domains meant
restarting the dev server, so working across a portfolio meant running one
board per domain and keeping track of which tab was which.

That is a UI problem, not a domain-model one. The board is a viewer; nothing
about showing eight domains in one window asks them to share a root.

## Decision

**The board is a singleton over N daemons, and the domain is in the URL.** One
instance lists every configured domain in a narrow left rail and addresses
each as `/d/{slug}/…`, so a link, a bookmark and the back button all name a
domain rather than inheriting whichever one was last selected. Its API client
is built per domain over that domain's own origin; query caches are keyed by
slug, so two domains can never show each other's cards.

**Nothing is aggregated.** The rail is a switcher. A tile may say whether the
daemon behind it answers and how many questions are waiting there — facts
about the *process*, read from the domain's own `/api/status` — but no page
merges one domain's content into another's. Decision 0002 is untouched: a
portfolio view is still a holdco domain's job, not a viewer's.

**The list of domains belongs to the operator, not to any domain.** It lives
at `~/.trellis/board.toml` — outside every root, because no root may know
about its siblings. An entry names a domain by `root` (the endpoint is read
from that root's live `serve.addr`, falling back to its `runtime.toml`) or by
an explicit `url` for one served elsewhere. Naming the root means the port is
declared once, where it already was.

**`trellis serve` answers loopback origins.** A board served from one origin
reading a daemon on another is cross-origin by construction, so the surface
now sends `Access-Control-Allow-Origin` — echoing the request's origin, and
only when its host is `localhost`, `127.0.0.1` or `[::1]`, at any port. Every
other origin gets exactly what it got before: a reply the browser refuses to
hand the page. This is the same reasoning that makes the bind address
loopback by default (there is no authentication here), applied to the same
boundary. No `Allow-Credentials` — there is no cookie or header to ride, and
saying so would only invite one.

The alternative was to front the daemons with a proxy so the browser only ever
saw one origin. It was rejected as a second serving process to run and keep
configured, for a single response header — and a proxy would have to hold
credentials for every remote domain, concentrating exactly what sovereign
roots keep apart.

## Consequences

- The board's dev server proxies nothing; `TRELLIS_API` is gone. It serves the
  SPA and resolves `~/.trellis/board.toml` at `GET /board.json`, re-read per
  request, because resolving a root's endpoint needs a filesystem the browser
  does not have.
- Adding a domain is three lines in one file. No root is edited, no daemon
  restarted, and no `runtime.toml` gains a key — CORS is default-on for
  loopback, so the eight existing roots need nothing.
- A domain with no daemon running is the ordinary case, not an error: its tile
  dims and the rest of the board keeps working.
- Exposing a daemon beyond loopback is still a deliberate act, and a board on
  another machine still cannot read it. That is the same bargain as before,
  and when a domain does move to its own host it will need real
  authentication — at which point the origin rule is the smaller half of the
  problem.
