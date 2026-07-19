# Evals

Evals for the plugin's prompt-borne members. Trellis's own ontology names this
layer — a holder package carries `evals/`, "mandate compliance as test cases"
(spec hierarchy), and "an eval is a requirement on an agent made testable"
(`spec/patterns.md`). The plugin is the home of its portable agent identities,
so their evals live here (decision 0025).

## Layout

The harness is cross-cutting; the graded substance is per-suite:

- `run.sh`, `prepare.mjs`, `grade.mjs` — the shared machinery: compose, date
  substitution, headless run, deterministic grading.
- `skeleton/` — the universal Trellis-root boilerplate every fixture composes
  on: `conventions.md`, the `org/founder` owner role, `decisions/0000`,
  `.gitignore`. Edit a convention once, every fixture follows.
- `fixtures/` — shared fixtures meaningful to any suite. Today: `healthy`, a
  clean root (trailmix, hiking meal kits) on which any member's eval can
  expect silence — the universal precision guard.
- `{suite}/` — one directory per evaled member (today: `focus/`):
  - `overlay/` — the suite's role(s) and `rituals.md` row, composed over the
    skeleton;
  - `fixtures/{name}/` — purpose-built fixture *deltas*: only the authored
    substance (market, strategy, problem, metrics, plans) that seeds the
    conditions this suite grades;
  - `contracts/{name}.yaml` — the grading contract per fixture; the contract
    list is the suite's fixture set, and shared fixtures join a suite by
    getting a contract here. Contracts never enter the composed root.
- `results/` — run output and stderr per fixture (gitignored).

**Why defect fixtures stay per-suite.** A fixture's value is that everything
except its seeded conditions is clean — that is what makes its contract
trustworthy. A fixture shared between two suites becomes edit-frozen: seeding a
new condition for one suite risks breaking the other's expectations. Machinery,
skeleton, and clean roots are cross-cutting; seeded defects are the suite's
authored substance and are never shared.

## Running

```sh
./run.sh focus                       # every fixture the focus suite has a contract for
./run.sh focus healthy               # one fixture
./run.sh focus --ritual value-dead   # enter via /trellis:ritual — the full
                                     # scheduled path: act, mandate, delegation
```

Per fixture, `run.sh` composes skeleton → overlay → delta into a temp root,
substitutes the date tokens (`%%TODAY%%`, `%%DAYS_AGO_N%%`, `%%DAYS_AHEAD_N%%`
— fixtures stay static while horizons and freshness windows keep firing),
git-inits with a backdated commit (status ages), runs one headless session
(capped `--max-budget-usd 5`), and grades. The entry command defaults to
`/trellis:{suite}` (`/trellis:ritual {suite}` with `--ritual`); a suite can
override via `{suite}/suite.env`.

## The grading contract

`grade.mjs` extracts the fenced-YAML finding records from the run output (the
record shape is defined in `checks/plan-effectiveness.md` and travels verbatim
through every relay) and checks:

- `must_find` — entries of kind, optional checklist `item`, and
  `refs_include` substrings (`a|b` means either satisfies the entry);
- `must_not_flag` — substrings no finding may ref;
- `max_findings` — optional cap (clean-root fixtures pin it to 0);
- the never-writes boundary — `git status --porcelain` empty in the composed
  root after every run.

Grading is deterministic on kind / item / refs and the no-writes check;
evidence and action quality stay human-judged — read `results/{suite}-{name}.out.md`
when a judgment call looks off. Findings emitted outside fenced `yaml` blocks
are invisible to the grader: that is the output contract, not a gap.

## Adding a suite

Create `{suite}/contracts/` (the runner's marker) with one contract per
fixture, `{suite}/fixtures/` for its purpose-built deltas, and `{suite}/overlay/`
for its role and rituals row. Give shared fixtures a contract to include them.
The runner is manual — no CI, per the staging philosophy in `spec/runtime.md`:
extracted by pull, not built ahead.
