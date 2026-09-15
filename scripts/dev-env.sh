#!/usr/bin/env bash
# Source this file in Bash to use the repository-local development tools.
callgrind_repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
export CALLGRIND_DEV_DIR="${CALLGRIND_DEV_DIR:-$callgrind_repo_root/.dev}"
export CARGO_HOME="$CALLGRIND_DEV_DIR/cargo"
export RUSTUP_HOME="$CALLGRIND_DEV_DIR/rustup"
export PATH="$CARGO_HOME/bin:$PATH"
