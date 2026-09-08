#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(git -C "$PROJECT_DIR" rev-parse --show-toplevel)"
PROJECT_PREFIX="${PROJECT_DIR#"$REPO_ROOT"/}"
[ "$PROJECT_PREFIX" = "$PROJECT_DIR" ] && PROJECT_PREFIX="."

staged_files() {
  git -C "$REPO_ROOT" diff --cached --name-only --diff-filter=ACMR -- "$PROJECT_PREFIX"
}

if [ -z "$(staged_files)" ]; then
  exit 0
fi

echo "pre-commit: checking $PROJECT_PREFIX"
make -C "$PROJECT_DIR" lint types worker-check

if staged_files | grep -q '\.py$'; then
  make -C "$PROJECT_DIR" test
fi

