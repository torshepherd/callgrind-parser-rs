#!/usr/bin/env bash
set -euo pipefail
source "$(dirname -- "${BASH_SOURCE[0]}")/dev-env.sh"
cd "$callgrind_repo_root"

# Other platforms can use their own rustup with rust-toolchain.toml.
if [[ "$(uname -s)/$(uname -m)" != Linux/x86_64 ]]; then
  echo "Bootstrap supports x86_64 Linux; use rustup and Cargo directly elsewhere." >&2
  exit 2
fi
for tool in curl sha256sum tar cc sed; do
  command -v "$tool" >/dev/null || { echo "Required host tool missing: $tool" >&2; exit 2; }
done

mkdir -p "$CARGO_HOME/bin" "$CALLGRIND_DEV_DIR/downloads"
download() {
  local url="$1" destination="$2" expected="$3"
  if [[ ! -f "$destination" ]] || ! printf '%s  %s\n' "$expected" "$destination" | sha256sum --check --status; then
    curl --proto '=https' --tlsv1.2 --fail --location --silent --show-error \
      --connect-timeout 20 --max-time 300 --output "$destination.partial" "$url"
    printf '%s  %s\n' "$expected" "$destination.partial" | sha256sum --check --status
    mv "$destination.partial" "$destination"
  fi
}

if [[ ! -x "$CARGO_HOME/bin/rustup" ]]; then
  installer="$CALLGRIND_DEV_DIR/downloads/rustup-init-1.29.1"
  download \
    https://static.rust-lang.org/rustup/archive/1.29.1/x86_64-unknown-linux-gnu/rustup-init \
    "$installer" dda7234360b7f578ca8b0ddcb80145646fa61a67c1720a5abc7051b35c9fcb71
  chmod u+x "$installer"
  "$installer" -y --no-modify-path --default-toolchain none
fi

# Do not rely on rustup's deprecated implicit installation behavior.
toolchain="$(sed -n 's/^channel = "\([^"]*\)"$/\1/p' rust-toolchain.toml)"
[[ -n "$toolchain" ]] || { echo "Missing pinned Rust channel" >&2; exit 2; }
if ! rustup run "$toolchain" rustc --version >/dev/null 2>&1 || \
   ! rustup run "$toolchain" cargo fmt --version >/dev/null 2>&1 || \
   ! rustup run "$toolchain" cargo clippy --version >/dev/null 2>&1; then
  rustup toolchain install "$toolchain" --profile minimal --component rustfmt \
    --component clippy --no-self-update
fi
cargo --version
rustc --version
cargo fmt --version
cargo clippy --version

nextest_version=0.9.144
if [[ ! -x "$CARGO_HOME/bin/cargo-nextest" ]] || \
   [[ "$(cargo nextest --version)" != "cargo-nextest $nextest_version "* ]]; then
  archive="$CALLGRIND_DEV_DIR/downloads/cargo-nextest-$nextest_version.tar.gz"
  download \
    "https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-$nextest_version/cargo-nextest-$nextest_version-x86_64-unknown-linux-musl.tar.gz" \
    "$archive" 20ed0a7d3d6f8dda9bb1b0bcb5838aea5784d3e2360746280868996d709dde0a
  tar --no-same-owner --no-same-permissions -xzf "$archive" -C "$CARGO_HOME/bin" cargo-nextest
fi
cargo nextest --version

# Resolve/download once; subsequent development checks can use --offline.
cargo fetch --locked
echo "Ready. Run scripts/cargo.sh nextest run --workspace --locked --offline"
echo "Or: source scripts/dev-env.sh; cargo test --workspace --locked --offline"
