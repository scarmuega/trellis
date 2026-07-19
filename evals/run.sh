#!/usr/bin/env bash
# Compose and run eval fixtures for a plugin-member suite.
#
#   ./run.sh <suite> [--ritual] [fixture ...]
#
# A run composes three layers into a temp root — skeleton/ (universal
# Trellis-root boilerplate), <suite>/overlay/ (the suite's role and rituals),
# then the fixture delta — substitutes date tokens, git-inits with a backdated
# commit, runs the suite's entry headlessly, and grades against
# <suite>/contracts/<fixture>.yaml. Fixture deltas resolve from
# <suite>/fixtures/ first, then the shared fixtures/; the default fixture list
# is whatever the suite has contracts for.
#
# Entry defaults to /trellis:<suite>, or "/trellis:ritual <suite>" with
# --ritual; a suite may override both via <suite>/suite.env (ENTRY,
# RITUAL_ENTRY). Requires claude on PATH (with its API key), node, git.
# Exit 0 = every fixture passed.
set -euo pipefail

EVAL_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PLUGIN_DIR="$(cd "$EVAL_DIR/.." && pwd)"
RESULTS_DIR="$EVAL_DIR/results"

SUITE="${1:-}"
if [[ -z "$SUITE" || ! -d "$EVAL_DIR/$SUITE/contracts" ]]; then
  echo "usage: run.sh <suite> [--ritual] [fixture ...]" >&2
  echo "suites: $(ls -d "$EVAL_DIR"/*/contracts 2>/dev/null | xargs -n1 dirname | xargs -n1 basename | tr '\n' ' ')" >&2
  exit 2
fi
shift

ENTRY="/trellis:$SUITE"
RITUAL_ENTRY="/trellis:ritual $SUITE"
if [[ -f "$EVAL_DIR/$SUITE/suite.env" ]]; then
  # shellcheck disable=SC1090
  source "$EVAL_DIR/$SUITE/suite.env"
fi

if [[ "${1:-}" == "--ritual" ]]; then
  ENTRY="$RITUAL_ENTRY"
  shift
fi

if [[ $# -gt 0 ]]; then
  FIXTURES=("$@")
else
  FIXTURES=()
  for contract in "$EVAL_DIR/$SUITE/contracts"/*.yaml; do
    FIXTURES+=("$(basename "$contract" .yaml)")
  done
fi

mkdir -p "$RESULTS_DIR"
BACKDATE="$(node -e 'console.log(new Date(Date.now() - 60 * 86400000).toISOString())')"
overall=0

for name in "${FIXTURES[@]}"; do
  contract="$EVAL_DIR/$SUITE/contracts/$name.yaml"
  delta="$EVAL_DIR/$SUITE/fixtures/$name"
  [[ -d "$delta" ]] || delta="$EVAL_DIR/fixtures/$name"
  if [[ ! -d "$delta" || ! -f "$contract" ]]; then
    echo "no fixture delta or contract for: $name" >&2
    overall=1
    continue
  fi

  work="$(mktemp -d)/root"
  mkdir -p "$work"
  cp -R "$EVAL_DIR/skeleton/." "$work/"
  if [[ -d "$EVAL_DIR/$SUITE/overlay" ]]; then
    cp -R "$EVAL_DIR/$SUITE/overlay/." "$work/"
  fi
  cp -R "$delta/." "$work/"
  node "$EVAL_DIR/prepare.mjs" "$work"
  git -C "$work" init -q
  git -C "$work" add -A
  GIT_AUTHOR_DATE="$BACKDATE" GIT_COMMITTER_DATE="$BACKDATE" \
    git -C "$work" -c user.name=trellis-evals -c user.email=evals@localhost \
    commit -qm "fixture: $name"

  out="$RESULTS_DIR/$SUITE-$name.out.md"
  echo "── $SUITE/$name ($ENTRY)"
  (cd "$work" && claude -p "$ENTRY" \
    --plugin-dir "$PLUGIN_DIR" \
    --permission-mode acceptEdits \
    --allowedTools "Read" "Grep" "Glob" "Task" "Bash(git *)" \
    --max-budget-usd 5) >"$out" 2>"$RESULTS_DIR/$SUITE-$name.err.log" || true
  node "$EVAL_DIR/grade.mjs" "$out" "$contract" "$work" || overall=1
done

exit $overall
