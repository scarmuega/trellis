# trellis board

A web UI for visualizing and acting on trellis artifacts, backed by the
read-only HTTP surface of `trellis serve`.

## Stack

- **Vite + React + TypeScript** — static SPA; the built `dist/` can be embedded
  into the `trellis` binary the same way `serve` embeds its current inline page.
- **@tanstack/react-query** — polling and caching over the `/api/*` endpoints.
- **Tailwind CSS v4** (+ typography plugin for rendered markdown).
- **react-markdown + remark-gfm** — artifact and view bodies arrive as raw
  markdown with GFM tables.
- **@xyflow/react (React Flow) + elkjs** — node/edge graphs; elkjs provides
  layered DAG layout once real edges (derivation, awaits) are wired in.
- **react-router** — client-side routing.

## Many domains, one board

A domain is a sovereign root with its own daemon on its own port (decisions
0002, 0046). The board is a singleton over all of them (decision 0062): a
narrow left rail lists every configured domain, the slug rides in the path
(`/d/{slug}/plans`), and each domain's API client is built over that domain's
own origin. Nothing is aggregated — the rail switches, it does not merge.

The list is yours, not any domain's, so it lives outside every root at
`~/.trellis/board.toml` (override with `TRELLIS_BOARD_CONFIG`):

```toml
[[domains]]
slug  = "txpipe"
label = "TxPipe"          # optional, defaults to the slug
root  = "~/Brain/txpipe"  # endpoint read from that root

[[domains]]
slug  = "tx3"
url   = "http://127.0.0.1:7366"   # or name the address outright
badge = "T3"                      # optional two characters for the tile
```

Naming a `root` means the port is declared once, where it already was: the
endpoint comes from that root's live `.trellis/runtime/serve.addr`, falling
back to its `runtime.toml` `[server]`. Use `url` for a domain served
elsewhere. Saving the file reloads the page.

## Development

Run a daemon in each domain root you want to see, then the dev server:

```sh
trellis serve -C ~/Brain/txpipe   # binds what its runtime.toml says
trellis serve -C ~/Brain/tx3
npm run dev                       # one board, both domains
```

Nothing is proxied — the board reads each daemon directly, cross-origin, which
`trellis serve` answers for loopback origins only. A domain with no daemon
running is not an error: its tile dims and the rest of the board keeps working.

## Layout

- `vite/domains.ts` — resolves `board.toml` and serves it at `/board.json`
- `src/lib/domains.ts` — the resolved list, as the browser reads it
- `src/lib/domain.tsx` — which domain the page is showing: its client, its
  link prefix, its query-key scope
- `src/components/DomainRail.tsx` — the switcher, one tile per domain, each
  polling its own `/api/status` for liveness and waiting questions
- `src/lib/api.ts` — typed client for the serve API, one per domain origin
- `src/pages/plans/` — the Plans section (`/plans`), one dataset with two views:
  - `PlansPage.tsx` — section shell with the Board | Graph toggle
  - `BoardView.tsx` — kanban columns from `/api/board`
  - `GraphView.tsx` — the `awaits:` DAG: plans as nodes (status-styled), awaits edges laid out left-to-right by elkjs; the edge holding a plan is drawn red, and an awaits target that names no plan renders as a dashed "missing" node
  - `PlanDrawer.tsx` — right drawer showing a plan's facts and markdown; addressed by the `?plan=` search param so it opens over either view and survives refresh. Its **Errand** menu is the operator's ask over one plan (`POST /api/plans/{slug}/errand`, decisions 0048 and 0060): an instruction written here, sized at a model and effort taken from the daemon's own resolved tiers, relayed by serve to the running dispatcher; disabled with the reason when no dispatcher runs or a session is already on the plan. Its **status chip** is a menu of the plan's legal lifecycle moves (`POST /api/plans/{slug}/status`, decision 0049) — the same guarded flips as `trellis plan release | claim | unblock | retire`, readiness refusals offering the CLI's force override
- `src/pages/ArtifactPage.tsx` — any artifact rendered as markdown, from `/api/artifacts/{rel}`
