#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/dev-env.sh"
cd "$callgrind_repo_root"
if [[ ! -x "$CARGO_HOME/bin/cargo" ]]; then
  echo "Run bash .codex/setup.sh first (no root or Nix needed)." >&2
  exit 127
fi
exec "$CARGO_HOME/bin/cargo" "$@"
