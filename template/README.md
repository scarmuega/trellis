# Trellis domain template

Copy this directory to scaffold a new domain root:

    cp -r template/ path/to/{your-domain}

Then:
1. Fill the founding map first: needs (`## N-{slug}`), market, and personas in
   `market.md` — technology-free; it must survive any pivot.
2. Write your first strategy: rename `strategy/first-strategy.md`, point its
   `need:` at a `market.md` anchor, and flip `status:` to `committed` once it is
   the model as operated (only committed induces).
3. Map what the strategy induces: one `problem/{subdomain}.md` per subdomain, each
   with an `induced-by:` edge back to the strategy, `class:` on the edge.
4. Fill `brand.md` and `economics.md`.
5. Replace every `<owner>` placeholder with a real `org/{role}` ref.
6. Review `conventions.md` — it is YOUR instance's authoritative copy; adjust the
   tag registry, plan types, and boundary guarantees to your tooling.
7. Record `decisions/0000-adopt-trellis.md` (pre-written; pin your spec version).
8. Wire the runtime (see `conventions.md` → Runtime binding): add
   `ANTHROPIC_API_KEY` to the forge's secrets, align
   `.github/workflows/rituals.yml` with your `rituals.md` cadences, pin the
   plugin checkout ref in both workflows, create a `role:{name}` label per role
   reachable from ingress, and protect core-class paths (branch protection +
   CODEOWNERS).
9. Delete this file.
