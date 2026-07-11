---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0010 — Plugin members are bare-named under the `trellis:` namespace

## Context
The base skill was first named `trellis-conventions` to be conflict-safe as a
standalone install (extending the retrieval-key discipline of 0005). Once it became
a member of the `trellis` plugin (0009), Claude Code namespaces every skill and
agent by plugin — invoked as `trellis:{name}` — which makes a self-prefix
redundant: `trellis:trellis-conventions` stutters.

## Decision
The bare token `trellis` is the plugin/namespace and is **never** a member name.
Skills and agents are named by their bare topic or role — `conventions`,
`scaffold`, `lint`, `steward` — and surface as `trellis:conventions`,
`trellis:steward`, and so on. `trellis-conventions` is renamed to `conventions`.
The rule is documented in the README's "Extending the plugin" section.

## Consequences
Members are only ever consumed through the plugin, so there is no bare-name
collision to defend against — the namespace does that work. New skills and agents
add no prefix. This is the vercel-plugin convention (clean plugin token → bare
member names).

## Alternatives rejected
Self-namespaced member names (`trellis-conventions`, `trellis-scaffold`) — the
session's earlier position, reversed here; redundant once the plugin supplies the
prefix. The tx3 self-prefix pattern (`tx3:check` inside plugin `tx3-skills`) applies
only when the plugin name is not the desired prefix, which is not our case. Retained
from the earlier decision only its durable half: the bare `trellis` token stays
reserved for the namespace.
