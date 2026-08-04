# Trellis domain template

Copy this directory to scaffold a new domain root:

    cp -r template/ path/to/{your-domain}

Then:
1. Fill the founding map first: needs (`## N-{slug}`), market, and personas in
   `market.md` — technology-free; it must survive any pivot.
2. Write your first strategy: rename `strategy/first-strategy.md`, point its
   `need:` at a `market.md` anchor, and advance `status:` to `validated` when
   you commit to operating it — the committed band (`validated` and beyond)
   induces and owes its `funded-by:` declaration (what sustains it: `self`, a
   capture strategy, or an external ref).
3. Map what the strategy induces: one `problem/{subdomain}.md` per subdomain, each
   with an `induced-by:` edge back to the strategy, `class:` on the edge.
4. Fill `brand.md` and `economics.md`.
5. Replace every `<owner>` placeholder with a real `org/{role}` ref.
6. Review `conventions.md` — it is YOUR instance's authoritative copy; adjust the
   tag registry, plan types, and boundary guarantees to your tooling.
7. Record `decisions/0000-adopt-trellis.md` (pre-written; pin your spec version).
8. Wire the runtime (see `conventions.md` → Runtime binding): review
   `runtime.toml`, then run `trellis serve` at the root — it is the domain's
   clock, and its board and API are on `http://127.0.0.1:7357`.
   `ANTHROPIC_API_KEY` lives in the daemon's environment, never in this root.
   Protect core-class paths on your forge (branch protection plus a generated
   CODEOWNERS: `trellis view codeowners --write`).
9. Delete this file.
