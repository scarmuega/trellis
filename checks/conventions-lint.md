# Conventions lint checklist

Canonical convention-compliance checks for a Trellis domain root. Referenced by the
`trellis:steward` agent (its lint ritual) and summarized in the `trellis:conventions`
skill — this file is the single source; keep both pointing here rather than copying
the list. Each item is a pass/fail check across the root; each failure becomes an
escalation to the artifact's owner.

1. Every artifact has frontmatter with valid `provenance:` and `owner:`.
2. No `generated` artifact has hand-edits (diff against its generator's output
   where feasible; otherwise flag suspicious edits).
3. Every `problem/{subdomain}.md` declares at least one `induced-by:` edge; each
   edge names an existing `strategy/{strategy}.md` and a legal
   `class: core|supporting|generic`. No node-level `class:` remains.
4. Every plan declares a legal `status: draft|ready|active|blocked|retired` and a
   registered `type:`; its `subdomains:`, `contexts:`, `metrics:`, and
   `decisions:` refs resolve to existing anchors.
5. Every mandate declares `authority:` and `escalate-to:`; every role has a
   holder (package or ref).
6. Every agent holder *package* has at least one eval (a `ref.md` holder is exempt).
7. Terms of art used in artifacts exist in `glossary.md` (or a BC-local glossary).
8. Tags in use are registered in `conventions.md`.
9. `decisions/` is append-only: no accepted decision has been edited; supersedence
   is a new numbered file.
10. No secrets anywhere in the root (keys, tokens, credentials, account numbers).
11. No directory that is a grouping rather than a kind (spec rule 7). A
    root-level `skills/` is this violation by name: procedures live in
    `solution/{bc}/skills/` (business) or `org/{role}/holder/skills/` (agent
    package) — escalate with the Cross-cutting procedures pattern's resolution
    (convention, ritual, runtime concern, or missing bounded context). A
    `plans/{plan}/` directory is this violation by name too: sub-plans are flat
    siblings (`plans/{parent}-{piece}.md`) — escalate with the Plan
    decomposition pattern's resolution (umbrella plan + sibling pieces +
    family tag; a family needing its own goals and metrics is rule 8 — a new
    root).
12. `metrics/actuals/` content is within the freshness window defined by
    `rituals.md`.
13. Every `strategy/{strategy}.md` declares a legal
    `status: raw|defined|validated|implemented|established|discarded` (the
    maturity ladder — Strategy maturity pattern); its `need:` resolves to an
    `N-{slug}` need anchor in `market.md` (slug match — heading level
    and case don't matter); `supersedes:` (if present) resolves; committed-band
    strategies (`validated|implemented|established`) declare `differentiation:`.
14. Every subdomain has at least one edge to a committed-band strategy
    (`status: validated|implemented|established`); subdomains whose edges all
    point to strategies outside the committed band are
    orphans — escalate for re-parenting or archival (they carry core policy until
    resolved).
15. Any committed strategy with more than one `core` edge declares
    `core-ranking:` — a total order covering exactly its core subdomains.
16. `market.md` (the founding map) is technology-free: flag strategy vocabulary,
    chosen means, and product or stack names for relocation to `strategy/` or `solution/`
    (judgment check, like rule 2's "flag suspicious").
17. If any strategy is `discarded` and none is in the committed band, the pivot
    is incomplete — the root is operating nothing it can attribute. Escalate to
    the discarded strategy's owner: commit a successor or archive the induced
    subdomains.
18. The spec version pinned in `decisions/0000-adopt-trellis.md` matches the
    current Trellis spec version (the `vN` in the title of
    `${CLAUDE_PLUGIN_ROOT}/spec/model.md`). If the pin is behind, the root is
    operating against superseded conventions — escalate to the adopt-decision's
    owner to review the intervening spec changes and either re-pin (adopt the
    current version) or record a deviation decision for what it declines.
19. Every `funded-by:` edge is legal: `strategy:` is `self`, a resolving
    `strategy/{strategy}.md` ref, or a declared external ref (spec rule 2);
    `relation:` (if present) is `current` or `intended` — absent means
    `current`; no strategy names its own path (that is `self`). Edges to
    aspirational or discarded strategies are legal — they document a thesis or
    a loss — but never sustain.
20. Every committed-band strategy is economically sustained: at least one
    `current` edge whose target is `self`, a committed-band strategy, or an
    external ref. A committed strategy without one is an economic orphan —
    same spirit and severity as item 14's orphaned subdomain: escalate to its
    owner to re-fund it, commit the conversion it intends, or reconsider the
    commitment.
21. The committed band has a capture point: at least one committed strategy
    captures its own revenue (a `current` `self` edge) or is sustained from
    outside the root (an external ref). A band whose strategies are funded
    only by each other is a closed loop — value circulates, nothing enters.
    Escalate as with item 17: the root is operating nothing that pays for it.
