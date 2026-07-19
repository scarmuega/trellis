# Focus suite

Evals for the plan-effectiveness prompt: does `trellis:focus` find the seeded
defects, spare the healthy plans, and write nothing? Harness mechanics, the
composition model, and the contract format: `../README.md`. The checklist under
test: `checks/plan-effectiveness.md`.

## Fixtures

Five purpose-built deltas plus the shared clean root, each a small fictional
business:

| fixture | seeds items | fiction |
|---|---|---|
| `coverage-gaps` | 1–3 (gap) | brewkit — homebrew subscription boxes |
| `metric-drift` | 4–5 (challenge, blocker) | petalpost — same-day flowers |
| `misallocation` | 6–8 (candidate) | ledgerloom — automated bookkeeping |
| `blockers-risks` | 9–11 (blocker, risk) | campfire-crm — guiding-company CRM |
| `value-dead` | 12–13 (challenge) | inkwell — indie-author publishing |
| `healthy` (shared) | none — zero findings expected | trailmix — hiking meal kits |

`overlay/` supplies the `org/focus` role and the `focus` rituals row every
composed root needs.

## Suite-specific grading notes

Defect fixtures grade recall and tolerate legitimate co-firings — a coverage
gap is often also a misallocation, so `must_not_flag` is used only where an
artifact is genuinely unimplicated (e.g. `value-dead`'s `grow-direct-sales`
distractor). Precision is carried by `healthy` (`max_findings: 0`) and the
never-writes boundary is asserted on every run.
