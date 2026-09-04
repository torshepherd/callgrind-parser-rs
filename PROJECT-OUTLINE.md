# Project outline

## What this is

This repository is the starting point for a Rust suite that reads, converts,
annotates, and explores Valgrind Callgrind data. It is both a Cargo workspace
for product code and a Nix-based reference laboratory for testing Rust behavior
against the original Valgrind tools.

The initial conversation established two practical goals:

1. make local and automated testing reproducible on Linux; and
2. let a fresh ChatGPT/Codex environment resume development without relying on
   tools or state left behind in an earlier transient VM.

The project lives at `https://github.com/torshepherd/callgrind-parser-rs`.
The accompanying archive is a durable point-in-time handoff. The GitHub
Actions workflow itself is intentionally deferred. The repository is
structured so the eventual workflow can install Nix and run one command:
`nix flake check -L`.

## Cargo workspace

| Package | Kind | Intended responsibility |
| --- | --- | --- |
| `callgrind-parser` | Library | Parse Callgrind files into a shared data model and expose analysis primitives. |
| `callgrind-annotate` | Binary | Reimplement `callgrind_annotate`, with output checked against Valgrind's reference implementation. |
| `callgrind2pprof` | Binary | Convert Callgrind profiles into the pprof ecosystem's representation. |
| `textgrind` | Binary | Provide a terminal UI for interactive profile exploration as a lightweight KCachegrind alternative. |
| `webgrind` | Binary | Serve a web UI backed by the shared parser library, also as a KCachegrind alternative. |

All four consumers already depend on `callgrind-parser`, but they are only
compiling placeholders. The parser has one deliberately small real function
and unit test so the testing path is exercised from the beginning.

## Test model

Cargo owns the Rust dependency graph and Rust tests. `cargo nextest` is the
preferred runner for the workspace because it gives a fast, CI-friendly test
harness; ordinary `cargo test --workspace` remains a supported baseline.

Nix owns the toolchain, native dependencies, reference tools, and hermetic
integration fixtures. The current integration test:

- builds a deterministic SQLite database;
- runs a representative read workload with two SQLite page-cache sizes;
- profiles every workload with six explicit Callgrind configurations;
- retains 12 raw profiles and 12 reference `callgrind_annotate` reports; and
- verifies the expected profiles, events, summaries, annotations, and matrix
  row count before completing.

The Rust workspace and the SQLite/Callgrind matrix are separate Nix checks, so
the quick Rust loop stays quick. `nix flake check -L` runs both and is the
future CI contract.

As `callgrind-annotate` grows, the next important integration layer is a
differential test: feed one generated profile to both implementations,
normalize only documented presentation differences, and compare their parsed
or rendered results. The same fixture can later exercise `callgrind2pprof`,
`textgrind`, and `webgrind` at their appropriate boundaries.

## Development environments

The locked `flake.nix` and `flake.lock` define Rust, `cargo-nextest`, SQLite,
Valgrind, and the other command-line tools. Enter them with:

```console
./scripts/nix.sh develop
```

For a fresh ChatGPT/Codex Linux environment, extract this project and run:

```console
bash .codex/setup.sh
```

That script installs Nix when needed and realizes the locked development shell
during the setup phase. The project therefore does not rely on a previous VM's
scratch filesystem; the Git repository is the source of truth and the saved
archive is a standalone snapshot.

The eventual GitHub Actions job should use the same flake rather than recreate
the environment in YAML. Its conceptual steps are: check out the repository,
install Nix with flakes enabled, restore useful Nix/Cargo caches if desired,
and run `nix flake check -L`.

## Required checks

Before a change is considered healthy, run:

```console
./scripts/nix.sh develop -c cargo fmt --all --check
./scripts/nix.sh develop -c cargo clippy --workspace --all-targets --all-features -- -D warnings
./scripts/nix.sh develop -c cargo nextest run --workspace
./scripts/nix.sh flake check -L
```

The final command repeats the workspace tests inside a clean Nix build and runs
the full Callgrind matrix. This duplication is intentional: the first commands
provide a fast development loop, while the Nix check proves reproducibility.
