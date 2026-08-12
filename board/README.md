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

## Development

Run the daemon in some domain root, then the dev server:

```sh
trellis serve            # binds 127.0.0.1:7357 by default
npm run dev              # proxies /api → 127.0.0.1:7357 (see vite.config.ts)
```

To point the board at a different domain, serve that root instead — the board
has no domain of its own, it shows whatever daemon it is proxied to:

```sh
trellis serve -C ~/path/to/other-domain             # reuses port 7357, or
trellis serve -C ~/path/to/other-domain --port 7358 # run beside another daemon
TRELLIS_API=http://127.0.0.1:7358 npm run dev
```

## Layout

- `src/lib/api.ts` — typed client for the serve API
- `src/pages/plans/` — the Plans section (`/plans`), one dataset with two views:
  - `PlansPage.tsx` — section shell with the Board | Graph toggle
  - `BoardView.tsx` — kanban columns from `/api/board`
  - `GraphView.tsx` — the `awaits:` DAG: plans as nodes (status-styled), awaits edges laid out left-to-right by elkjs; the edge holding a plan is drawn red, and an awaits target that names no plan renders as a dashed "missing" node
  - `PlanDrawer.tsx` — right drawer showing a plan's facts and markdown; addressed by the `?plan=` search param so it opens over either view and survives refresh. Its **Act** menu requests a refine session (`POST /api/plans/{slug}/refine`, decision 0048) — canned instructions (simplify / split / refactor scope) or a custom one — relayed by serve to the running dispatcher; disabled with the reason when no dispatcher runs or a session is already on the plan. Its **status chip** is a menu of the plan's legal lifecycle moves (`POST /api/plans/{slug}/status`, decision 0049) — the same guarded flips as `trellis plan release | claim | unblock | retire`, readiness refusals offering the CLI's force override
- `src/pages/ArtifactPage.tsx` — any artifact rendered as markdown, from `/api/artifacts/{rel}`
