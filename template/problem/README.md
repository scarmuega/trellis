---
provenance: authored
owner: <owner>
---
# Problem space

The induced problem space: the subdomains a committed strategy drags into this
domain. Nothing here is primitive — every subdomain owes its existence to a
commitment and says which. The invariant layer it answers to (needs, market,
personas) lives in `market.md`, the founding map.

- One file per subdomain in this directory, `problem/{subdomain}.md`
- Each carries `induced-by:` edges naming the strategy that put it there, with a
  `class:` per edge — classification lives on the edge and drives automation
  policy (core > supporting > generic)
- A subdomain with no edge to a committed strategy is an orphan (the lint flags
  it); re-parent onto a surviving commitment or archive
