# Optional pprof exporter audit probes

These are reproducible research inputs for
[the pprof Callgrind audit](../../../docs/PPROF-CALLGRIND-AUDIT.md), not a
Callgrind-to-pprof converter. `probe.go` uses upstream's public profile writer
and reader, then invokes its unmodified CLI. It creates 12 profiles and runs
32 commands, preserving each profile, decoded representation, argv, stdout
and stderr. Assertions cover stack ambiguity and selected exporter quirks.
The assertions describe this pinned version, not behavior to copy into Rust.

The source-audit mode remains available. A separate native CI job also passes
`--converter` and `--roundtrip` to validate the Rust implementation. It does
not require Go for ordinary Rust development. Network access is
needed only to obtain the pinned source/toolchain and its locked Go modules;
the actual probes need no network or native profiled workload.

## Converter/KCachegrind CI gate

After Rust bootstrap, on x86_64 Linux with C++, pkg-config and Qt6Core development
files installed, run `bash scripts/check-pprof.sh "$PWD/results/pprof"` from the
repository root with a new empty results directory. It verifies archive hashes
for Go 1.27.1 and pprof, tests four upstream packages, runs 12 fixtures/32 upstream
probes, cross-reads 12 Rust gzip outputs, checks negative rejection in both modes,
and compares 22 converted files with unmodified KCachegrind
`764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14`.

`check_pprof.py` reuses the raw TSV comparator and independently checks known
sample paths, self costs and edge weights. GUI inclusivity/cycle heuristics are
not the oracle. Exports and diagnostics survive failures. Unlike the hermetic
SQLite/Nix gate, this pins Go/reader sources but uses Ubuntu's host Qt/C++ packages.

For reverse-conversion checks, pass the directory of 12 raw SQLite files as
the optional second argument to `scripts/check-pprof.sh`. CI downloads the
successful SQLite job's retained corpus, selecting its latest available attempt
so rerunning only the pprof job works too. Both converters see the same raw bytes
used by the parser/annotator gate; no absolute host-specific totals are goldens.

`reverse.go` independently reads the generated gzip with Go pprof, checks exact
per-event sums and original function/source costs against the Rust production
parser's TSV, validates labels/identities and the absence of invented mappings
or stacks, then runs a real upstream `-top` report. Seven focused files (including
two multipart files), 22 forward outputs and 12 SQLite profiles yield 41 files /
43 part exports. The source corpus, parser TSV, gzip, top reports and diagnostics
are retained. `reverse_test.go` verifies rejection of corrupted costs, identities,
source attribution, events, stacks and addresses. Ordinary Cargo builds require
neither Go nor Qt.

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
