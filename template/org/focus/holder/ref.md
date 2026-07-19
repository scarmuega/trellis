---
provenance: authored
owner: <owner>
ref: trellis:focus
---
# Focus holder → trellis:focus

The holder for this role is the portable `trellis:focus` agent from the Trellis
plugin — authority is local, identity is portable (rule 4). The agent binds to
this root, reads this role's `../mandate.md` at run time, and acts only within
the authority it grants.

Requires the `trellis` plugin installed. To pin a domain-specific holder instead,
replace this ref with a package: a `system.md` plus at least one eval under `evals/`.
