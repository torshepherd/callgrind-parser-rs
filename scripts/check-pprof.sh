#!/usr/bin/env bash
# Separate native CI gate; ordinary Cargo builds do not require Go or Qt.
set -euo pipefail
[[ $# == 1 ]] || { echo "usage: $0 NEW_RESULTS_DIRECTORY" >&2; exit 2; }
repo_root=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$1"
out=$(cd "$1" && pwd)
[[ -z $(find "$out" -mindepth 1 -print -quit) ]] || { echo 'results directory must be empty' >&2; exit 1; }
build=$(mktemp -d "$repo_root/.dev/pprof-ci.XXXXXX")
pin=6331bc6350fe55a6fec2957299e0581dd7510e36
curl --fail --location --retry 3 https://go.dev/dl/go1.27.1.linux-amd64.tar.gz -o "$build/go.tar.gz"
printf '%s  %s\n' 63d339f0da5ab53635a56f2490a7984dfe12dfcff22ad749f63edaf590168445 "$build/go.tar.gz" | sha256sum -c -
tar -xzf "$build/go.tar.gz" -C "$build"
curl --fail --location --retry 3 "https://codeload.github.com/google/pprof/tar.gz/$pin" -o "$build/pprof.tar.gz"
printf '%s  %s\n' ab04d71d3ba750fe102f1887c67428cc01b7a75fc3d21a3a8f145b19de327630 "$build/pprof.tar.gz" | sha256sum -c -
tar -xzf "$build/pprof.tar.gz" -C "$build"
cmp "$build/pprof-$pin/proto/profile.proto" "$repo_root/crates/pprof-profile/profile.proto"
export GOTOOLCHAIN=local GOPATH="$build/gopath" GOCACHE="$build/gocache"
export PATH="$build/go/bin:$PATH"
cd "$build/pprof-$pin"
go mod download
go mod verify
export GOPROXY=off
go test ./internal/report ./internal/graph ./internal/driver ./profile
go build -o "$build/pprof" .
cd "$repo_root"
./scripts/cargo.sh build --workspace --bins --examples --locked --offline
cd "$build/pprof-$pin"
go run "$repo_root/tests/reference/pprof-audit/probe.go" --pprof "$build/pprof" --out "$out" \
  --converter "$repo_root/target/debug/pprof2callgrind" --roundtrip "$repo_root/target/debug/examples/roundtrip"
cd "$repo_root"
git init "$build/kcachegrind"
git -C "$build/kcachegrind" fetch --depth 1 https://github.com/KDE/kcachegrind.git 764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14
git -C "$build/kcachegrind" checkout --detach FETCH_HEAD
bash tests/reference/build-kcachegrind.sh "$build/kcachegrind" "$build/kcachegrind-export"
python3 tests/reference/check_pprof.py "$out" "$repo_root/target/debug/examples/reference_export" "$build/kcachegrind-export"
