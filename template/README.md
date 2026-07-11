# Trellis domain template

Copy this directory to scaffold a new domain root:

    cp -r template/ path/to/{your-domain}

Then:
1. Fill the founding map first: needs (`## N-{slug}`), market, and personas in
   `problem/README.md` — technology-free; it must survive any pivot.
2. Write your first strategy: rename `strategy/first-strategy.md`, point its
   `need:` at a founding-map anchor, and flip `status:` to `committed` once it is
   the model as operated (only committed induces).
3. Map what the strategy induces: one `problem/{subdomain}.md` per subdomain, each
   with an `induced-by:` edge back to the strategy, `class:` on the edge.
4. Fill `brand.md` and `economics.md`.
5. Replace every `<owner>` placeholder with a real `org/{role}` ref.
6. Review `conventions.md` — it is YOUR instance's authoritative copy; adjust the
   tag registry, plan types, and boundary guarantees to your tooling.
7. Record `decisions/0000-adopt-trellis.md` (pre-written; pin your spec version).
8. Delete this file.
