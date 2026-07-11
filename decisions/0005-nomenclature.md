---
provenance: authored
status: accepted
date: 2026-07-11
---
# 0005 — Nomenclature: names are retrieval keys

## Context
Every name in the tree lands in an agent's context window as a retrieval key.
Ambient tokens (`system`, `model`, `class`) and opaque acronyms (`prd`, `kpi`,
`bc`) degrade retrieval for agents and comprehension for humans.

## Decision
Naming principle (spec rule 10): distinctive tokens over ambient ones; expand,
don't abbreviate. Renames applied: `prd`→`problem`, `system`→`solution`,
`model.md`→`economics.md`, `kpi`→`metrics`, `orgchart`→`org`, `{entry}`→`{role}`,
`impl`→`holder`; frontmatter `class:`→`provenance:` (freeing `class:` for
core/supporting/generic), `bc:`→`contexts:`, `kpis:`→`metrics:`.

## Consequences
Top level reads as a sentence: problem, brand, economics, metrics, plans,
decisions, solution, org. DDD-canonical terms (glossary, bounded context,
context-map, contracts) and agent-ecosystem terms (skills, tools, evals,
system.md) are deliberately kept — imported meaning outweighs purity.

## Alternatives rejected
Strict kind-naming (`subdomains/`, `contexts/`) — problem/solution are DDD's
ontological spaces, not groupings; rule 7 targets groupings.
