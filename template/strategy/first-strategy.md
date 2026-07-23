---
provenance: authored
owner: <owner>
status: raw             # advance on evidence: raw → defined → validated (the
                        # commitment line — induction starts) → implemented →
                        # established; discarded is terminal
need: market.md#n-{slug}
differentiation: <why we win, against which alternative — one line>
funded-by:              # what sustains this strategy — owed at the commitment
  - strategy: self      # line: self (captures its own revenue), or the capture
                        # strategy's ref (+ relation: intended for a conversion
                        # thesis); economics.md narrates these edges
core-ranking: []        # required iff >1 core edge under this strategy;
                        # total order, scarcest attention first
---
# {strategy} — rename this file to the strategy's name

The committed solution to the need above: the business model as actually operated,
not as aspired to.

- The need addressed — ref the `## N-{slug}` anchor in the frontmatter
- The chosen means — how this business fulfills that need
- The differentiation claim — where this strategy competes, expanded from the
  frontmatter one-liner
- What sustains it — the `funded-by:` edges expanded: who pays, through which
  strategy, and any conversion thesis (`intended` edges) with its horizon
- What it induces — the subdomains this commitment drags into the domain; each
  gets a `problem/{subdomain}.md` with an `induced-by:` edge back to this file
