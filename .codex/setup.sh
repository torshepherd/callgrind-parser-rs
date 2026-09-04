#!/usr/bin/env bash
set -euo pipefail

if ! command -v nix >/dev/null 2>&1 && [[ ! -x /nix/var/nix/profiles/default/bin/nix ]]; then
  curl --proto '=https' --tlsv1.2 -sSf -L https://install.determinate.systems/nix \
    | sh -s -- install linux \
        --extra-conf "sandbox = false" \
        --init none \
        --no-confirm
fi

bash scripts/nix.sh develop --command true
