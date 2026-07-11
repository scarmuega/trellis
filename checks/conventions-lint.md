# Conventions lint checklist

Canonical convention-compliance checks for a Trellis domain root. Referenced by the
`trellis:steward` agent (its lint ritual) and summarized in the `trellis:conventions`
skill — this file is the single source; keep both pointing here rather than copying
the list. Each item is a pass/fail check across the root; each failure becomes an
escalation to the artifact's owner.

1. Every artifact has frontmatter with valid `provenance:` and `owner:`.
2. No `generated` artifact has hand-edits (diff against its generator's output
   where feasible; otherwise flag suspicious edits).
3. Every `problem/{subdomain}.md` declares `class: core|supporting|generic`.
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
11. No directory that is a grouping rather than a kind (spec rule 7).
12. `metrics/actuals/` content is within the freshness window defined by
    `rituals.md`.
