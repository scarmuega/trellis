# Changelog

All notable changes to the Trellis plugin are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

**Versioning.** While Trellis is single-maintainer and pre-1.0, the plugin omits an
explicit `version` and floats on the git commit SHA — every push to `main` is a new
version for installers (see `decisions/0012-plugin-versioning-float-on-sha.md`).
Once there are external consumers this switches to explicit
[Semantic Versioning](https://semver.org/): each release gets a `## [x.y.z]` section
here and a matching `vx.y.z` git tag.

## [Unreleased]

### Added

- Initial Trellis plugin prototype — the specification (`spec/trellis.md`), the
  domain scaffold (`template/`), the `trellis:conventions` skill, the
  `trellis:steward` agent, the shared lint checklist (`checks/conventions-lint.md`),
  and decision records 0001–0011.
- Self-distributing marketplace catalog (`.claude-plugin/marketplace.json`): the
  `trellis` plugin under the `scarmuega` marketplace, installable with
  `/plugin marketplace add scarmuega/trellis` then `/plugin install trellis@scarmuega`.
- Versioning policy (`decisions/0012`) and this changelog.
