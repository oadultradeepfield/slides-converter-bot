#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(git -C "$PROJECT_DIR" rev-parse --show-toplevel)"
HOOK_DIR="$(git -C "$PROJECT_DIR" rev-parse --git-path hooks)"
case "$HOOK_DIR" in /*) ;; *) HOOK_DIR="$REPO_ROOT/$HOOK_DIR" ;; esac
HOOK_FILE="$HOOK_DIR/pre-commit"

PROJECT_PREFIX="${PROJECT_DIR#"$REPO_ROOT"/}"
[ "$PROJECT_PREFIX" = "$PROJECT_DIR" ] && PROJECT_PREFIX="."
MARKER="slides-converter-bot: $PROJECT_PREFIX"

mkdir -p "$HOOK_DIR"
if [ ! -f "$HOOK_FILE" ]; then
  printf '#!/usr/bin/env bash\nset -euo pipefail\n' >"$HOOK_FILE"
fi
chmod +x "$HOOK_FILE"

if grep -qF "$MARKER" "$HOOK_FILE"; then
  echo "hook already installed for $PROJECT_PREFIX"
  exit 0
fi

{
  printf '\n# >>> %s >>>\n' "$MARKER"
  printf '"$(git rev-parse --show-toplevel)/%s/scripts/pre-commit.sh" || exit 1\n' "$PROJECT_PREFIX"
  printf '# <<< %s <<<\n' "$MARKER"
} >>"$HOOK_FILE"

chmod +x "$PROJECT_DIR/scripts/pre-commit.sh"
echo "installed pre-commit hook for $PROJECT_PREFIX"
