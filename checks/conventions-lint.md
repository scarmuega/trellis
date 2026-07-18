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
4. Every plan declares `status:` and a registered `type:`; its `subdomains:`,
   `contexts:`, `metrics:`, and `decisions:` refs resolve to existing anchors.
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
    (convention, ritual, runtime concern, or missing bounded context).
12. `metrics/actuals/` content is within the freshness window defined by
    `rituals.md`.
13. Every `strategy/{strategy}.md` declares a legal
    `status: aspirational|committed|retired`; its `need:` resolves to an
    `N-{slug}` need anchor in `market.md` (slug match — heading level
    and case don't matter); `supersedes:` (if present) resolves; committed
    strategies declare `differentiation:`.
14. Every subdomain has at least one edge to a `status: committed` strategy;
    subdomains whose edges all point to retired or aspirational strategies are
    orphans — escalate for re-parenting or archival (they carry core policy until
    resolved).
15. Any committed strategy with more than one `core` edge declares
    `core-ranking:` — a total order covering exactly its core subdomains.
16. `market.md` (the founding map) is technology-free: flag strategy vocabulary,
    chosen means, and product or stack names for relocation to `strategy/` or `solution/`
    (judgment check, like rule 2's "flag suspicious").
17. If any strategy is `retired` and none is `committed`, the pivot is
    incomplete — the root is operating nothing it can attribute. Escalate to the
    retired strategy's owner: commit a successor or archive the induced
    subdomains.
