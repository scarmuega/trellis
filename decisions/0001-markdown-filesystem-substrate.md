---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0001 — Versioned markdown filesystem as organizational substrate

## Context
The framework needs a medium that humans and AI agents are both native in. Code is
native to machines and opaque to most humans; SaaS tools and databases are opaque to
agents and fragment knowledge across vendors; meetings and chat are native to humans
and leave no structured trace.

## Decision
The organizational substrate is a git-versioned filesystem of natural-language
markdown with typed frontmatter. The documentation is the execution medium: the
artifact an agent loads is the process.

## Consequences
Knowledge becomes diffable, reviewable, and agent-loadable; belief revision carries
an audit trail for free. Fast-changing operational state does not belong in the
substrate and lives in external systems of record (see spec rule 5).

## Alternatives rejected
Database/SaaS-first (opaque to agents, vendor-fragmented); code-first (excludes
non-technical humans and problem-space knowledge).
