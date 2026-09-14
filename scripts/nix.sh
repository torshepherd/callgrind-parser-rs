#!/usr/bin/env bash
set -euo pipefail

if command -v nix >/dev/null 2>&1; then
  nix_bin="$(command -v nix)"
elif [[ -x /nix/var/nix/profiles/default/bin/nix ]]; then
  nix_bin="/nix/var/nix/profiles/default/bin/nix"
else
  echo "Nix is optional and not installed. Use scripts/cargo.sh for Rust development; install Nix separately for the integration matrix." >&2
  exit 127
fi

nix_args=(--extra-experimental-features "nix-command flakes")

# Root-only container installs do not use the multi-user nixbld group.
if [[ "$(id -u)" -eq 0 ]]; then
  nix_args+=(--option build-users-group "")
fi

exec "$nix_bin" "${nix_args[@]}" "$@"
