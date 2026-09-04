# Callgrind tools workspace

This project is a stubbed Rust workspace for a Callgrind parser and four tools,
plus a working reproducibility harness that pins SQLite, Valgrind/Callgrind, a
deterministic fixture, a realistic query workload, and a profiling matrix.

See [PROJECT-OUTLINE.md](PROJECT-OUTLINE.md) for the intended architecture and
development model.

## Workspace

- `callgrind-parser`: shared parser library
- `callgrind-annotate`: Rust port of the reference annotator
- `callgrind2pprof`: pprof converter
- `textgrind`: terminal profile explorer
- `webgrind`: parser-backed web profile explorer

The applications are placeholders, while the parser includes a minimal header
parser and test. Run the fast test loop with:

```console
./scripts/nix.sh develop -c cargo nextest run --workspace
```

Build and test the workspace hermetically with:

```console
./scripts/nix.sh build .#rustWorkspace -L
```

Run every Rust and integration check with:

```console
./scripts/nix.sh flake check -L
```

## SQLite + Callgrind reference matrix

Build only the reference integration fixture with:

```console
./scripts/nix.sh build .#smoke -L
```

That build succeeds only after it creates and validates 12 raw Callgrind files
and 12 reports from the reference `callgrind_annotate`. The result is available
through the `result` symlink, including `SUMMARY.tsv` and the exact command for
every run.

## What gets exercised

The fixture contains 5,000 customers and 50,000 purchases with deterministic
data and useful indexes. The read-only workload combines point probes, an
indexed join and range scan, covering-index aggregation, a window query, and
two broad scans.

Every Callgrind configuration runs with both a 64-page and 4,096-page SQLite
page cache. The Callgrind matrix is:

| Configuration | Simulation and collection |
| --- | --- |
| `ir-only` | Instruction reads only |
| `branches-and-jumps` | Branch prediction and jump relationships |
| `balanced-cache` | 32 KiB L1I/L1D and 8 MiB LL |
| `constrained-cache` | 16 KiB L1I/L1D and 512 KiB LL |
| `roomy-cache` | 64 KiB L1I/L1D and 16 MiB LL |
| `full-simulation` | Balanced caches, branches, jumps, syscall time, write-back, hardware prefetch, and cache-use metrics |

All simulated cache geometries are explicit. Callgrind's host autodetection is
intentionally avoided so GitHub Actions and other Linux VMs test the same
configuration.

## Development shell

```console
./scripts/nix.sh develop
cargo --version
sqlite3 --version
valgrind --version
```

To generate results into a normal writable directory while experimenting:

```console
fixture="$(./scripts/nix.sh build .#fixture --no-link --print-out-paths)"
./scripts/nix.sh develop -c ./scripts/run-matrix.sh \
  --fixture "$fixture" \
  --workload ./workload/workload.sql.in \
  --out ./results
```

## ChatGPT/Codex cloud environment

Set the environment setup command to:

```console
bash .codex/setup.sh
```

The script installs Nix in container-compatible, root-only mode when needed
and realizes the locked development environment while setup-phase network
access is available. Subsequent work can run
`./scripts/nix.sh build .#smoke -L` without
depending on whatever SQLite or Valgrind happens to be installed globally.

## Reproducibility boundary

`flake.lock` fixes the complete Nix package graph. The SQL data and profiling
options are deterministic, and the simulated cache geometries are explicit.
Raw event totals are deliberately not checked against golden numbers: the
same generic x86_64 user-space closure can still select different optimized
code paths on different CPUs. For a `callgrind_annotate` reimplementation,
compare the reference and replacement against the exact same raw profile.
