# Trellis domain template

Copy this directory to scaffold a new domain root:

    cp -r template/ path/to/{your-domain}

Then:
1. Fill the discovery half first: `problem/README.md`, `brand.md`, `economics.md`.
2. Replace every `<owner>` placeholder with a real `org/{role}` ref.
3. Review `conventions.md` — it is YOUR instance's authoritative copy; adjust the
   tag registry, plan types, and boundary guarantees to your tooling.
4. Record `decisions/0000-adopt-trellis.md` (pre-written; pin your spec version).
5. Delete this file.
