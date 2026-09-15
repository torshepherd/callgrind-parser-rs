# Optional pprof exporter audit probes

These are reproducible research inputs for
[the pprof Callgrind audit](../../../docs/PPROF-CALLGRIND-AUDIT.md), not a
Callgrind-to-pprof converter. `probe.go` uses upstream's public profile writer
and reader, then invokes its unmodified CLI. It creates 12 profiles and runs
32 commands, preserving each profile, decoded representation, argv, stdout
and stderr. Assertions cover stack ambiguity and selected exporter quirks.
The assertions describe this pinned version, not behavior to copy into Rust.

This is separate from ordinary Cargo/Python CI. It does not install a new
project dependency or require Go for Rust development. Network access is
needed only to obtain the pinned source/toolchain and its locked Go modules;
the actual probes need no network or native profiled workload.

## Reproduce from the repository root

Use Go 1.25 or later; the observed run used Go 1.27.1 linux-amd64.
[pprof-audit-sources.json](../../../docs/pprof-audit-sources.json) records its
official archive URL/hash and all audited source hashes. Install the toolchain
locally if necessary; do not substitute the potentially different pprof
vendored inside an arbitrary `go tool pprof`.

```bash
repo_root="$PWD"
pprof_audit_root="$repo_root/.dev/reference/pprof-audit"
mkdir -p "$pprof_audit_root"
curl --fail --location \
  https://codeload.github.com/google/pprof/tar.gz/6331bc6350fe55a6fec2957299e0581dd7510e36 \
  -o "$pprof_audit_root/pprof.tar.gz"
printf '%s  %s\n' \
  ab04d71d3ba750fe102f1887c67428cc01b7a75fc3d21a3a8f145b19de327630 \
  "$pprof_audit_root/pprof.tar.gz" | sha256sum -c -
tar -xzf "$pprof_audit_root/pprof.tar.gz" -C "$pprof_audit_root"
pprof_source="$pprof_audit_root/pprof-6331bc6350fe55a6fec2957299e0581dd7510e36"

export GOTOOLCHAIN=local
export GOPATH="$pprof_audit_root/gopath"
export GOCACHE="$pprof_audit_root/gocache"
cd "$pprof_source"
go mod download
go mod verify
GOPROXY=off go test ./internal/report ./internal/graph ./internal/driver ./profile
GOPROXY=off go build -o "$pprof_audit_root/pprof" .
probe_output=$(mktemp -d "$pprof_audit_root/probe.XXXXXX")
GOPROXY=off go run "$repo_root/tests/reference/pprof-audit/probe.go" \
  --pprof "$pprof_audit_root/pprof" --out "$probe_output"
cd "$repo_root"
```

The Go invocation deliberately runs inside the pinned upstream module, so its
`go.mod`/`go.sum` select dependencies without adding a second dependency system
to this repository. The program refuses nonempty output directories and gives
each CLI invocation a 30-second timeout. Do not run upstream golden-update
flags, edit upstream sources, or treat different pprof output as automatically
a regression in our code. Review an updated pin before updating expectations.

## Check the produced Callgrind with our parser

```bash
./scripts/cargo.sh build -p callgrind-parser --examples --locked --offline
./target/debug/examples/inspect "$probe_output/basic.stdout"
./target/debug/examples/reference_export "$probe_output/objects.stdout"
./target/debug/examples/reference_export "$probe_output/recursive-tree.stdout"
# Expected to fail with InvalidNumber: pprof emitted a negative counter.
if ./target/debug/examples/inspect "$probe_output/negative.stdout"; then
  echo 'Unexpected acceptance of negative Callgrind costs' >&2
  exit 1
fi
```

Expected observations:

- `basic`: self total 100, one zero-count call edge; no declared totals.
- `objects`: foo self 90 belongs to lib.so, but the emitted edge targets an
  app-qualified foo because pprof omitted `cob`. TSV identity fields are hex.
- `recursive-tree`: self belongs to `recur`, while callee names have `[1/2]`
  or `[2/2]` suffixes. They do not identify those self rows.
- `negative`: the unsigned production parser rejects the emitted -5.

With the unchanged Valgrind 3.26.0 `callgrind_annotate` installed, run:

```bash
callgrind_annotate --auto=no --threshold=100 --show-percs=no \
  "$probe_output/basic.stdout"
```

It calculates 190, not 100, because it misclassifies the `calls=0` edge cost as
self. That expected disagreement is described in both source audits. Never
adjust the generated count or Rust parser to make this particular reader pass.

The complete case table, source-derived explanations and reverse-conversion
implications live in the audit. Generated artifacts stay under `.dev/` and
are reproducible; do not commit binaries, Go caches or generated profiles.
