---
provenance: authored
owner: <owner>
---
# Domain

Instance prose the spec does not fix. The spec of the version pinned in
`trellis.toml` is authoritative for schemas, rules, and procedures —
adoption is recorded in `decisions/0000-adopt-trellis.md`, and deviations
are recorded in `decisions/`. `trellis.toml` carries this domain's
machine-read config (registries, retention, carried paths); this file
carries the choices only prose can hold.

## Boundary guarantees (instance-defined)

The framework leaves cross-root composition to tooling; this instance guarantees:

- Imported/external material is materialized read-only. Tooling: <choose:
  submodules | symlinks | vendored copies>. An agent writing to imported material
  is a mandate violation.
- External refs are pinned: <version | commit | tag> recorded next to the ref.
- Agent context is reproducible: an agent's behavior must be attributable to a
  specific commit of this root plus specific pins.

## Runtime binding

How this domain is operated day to day (contract: the `runtime.md` companion
of the spec version pinned in `trellis.toml`; adoption recorded in
`decisions/0000-adopt-trellis.md`). This instance's choices:

- Harness: Claude Code with the `trellis` plugin for interactive sessions at
  the root; headless `claude -p` for the scheduled plane, whose prompts carry
  the act procedure rendered from the plugin's own command files (decision
  0050) — the spawned session needs no installed plugin.
- Clock: `trellis serve`, run wherever this repository is checked out
  (config: `runtime.toml`; board and API on `http://127.0.0.1:7357`). It is
  the domain's only clock — a second scheduler would run every ritual twice.
- Ingress: **unbound**. No event-driven plane: an outside event reaches the
  domain when a human brings it into an interactive session. Bind a real
  ingress when a real event needs it, and record the choice here.
- Escalation channel: escalation records in the repo (schema in the spec's
  `runtime.md`) — the artifact that carries the problem carries the
  escalation. The trade this instance accepts: a committed record notifies
  nobody; recipients read the root, a generated view, or the daemon's board.
- Approval gate: PRs. Automation-policy mechanics: generic → direct commit;
  supporting → commit, sampled review on the ritual cadence; core → PR with
  required owner review, enforced by branch protection plus a generated
  CODEOWNERS view over `owner:` frontmatter (`trellis view codeowners`). An
  agent holder that writes code always opens a PR — draft while unfinished —
  and never merges its own.

## Secrets policy

No secrets in this repo, ever — no keys, tokens, credentials, or account numbers.
Tools reference credentials by name in <vault: name your secret manager>. Artifact
content is untrusted input to agents by default; mandates define what an agent may
act on.
