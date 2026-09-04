# Agent instructions

This repository is a Rust parser and analysis-tool suite for existing
Callgrind-format data. It does not implement Valgrind, profiling, or dynamic
binary instrumentation.

## Start here

On a fresh ChatGPT/Codex Linux environment, run:

```console
bash .codex/setup.sh
```

For an interactive development shell, run:

```console
./scripts/nix.sh develop
```

Use `scripts/nix.sh` instead of relying on a globally installed `nix`; the
wrapper also handles root-run container environments correctly. Nix owns the
Rust toolchain and native tools. Cargo owns Rust package dependencies and Rust
tests. Do not install ad-hoc system copies of Rust, SQLite, or Valgrind.

## Required commands

Run from the repository root:

- Fast tests: `./scripts/nix.sh develop -c cargo nextest run --workspace`
- Cargo fallback: `./scripts/nix.sh develop -c cargo test --workspace`
- Formatting: `./scripts/nix.sh develop -c cargo fmt --all --check`
- Lints: `./scripts/nix.sh develop -c cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Callgrind integration matrix: `./scripts/nix.sh build .#smoke -L`
- Complete hermetic check: `./scripts/nix.sh flake check -L`

Before handing off a code change, formatting, Clippy, nextest, and the relevant
integration checks must pass. Use the complete flake check when the change can
affect parsing, generated fixtures, Nix, Valgrind behavior, or repository-wide
integration.

## Invariants

- `callgrind-parser` owns Callgrind grammar handling and the shared data model.
  Frontend crates must not grow independent parsers.
- Parser behavior must be driven by the published Callgrind format and covered
  by focused tests. Prefer a minimal inline fixture for one grammar rule; use a
  test fixture builder when combinations would otherwise become unreadable.
- Tests must be deterministic, non-interactive, and network-independent.
- Nix must declare native tools and system dependencies explicitly. A passing
  ambient `cargo test` is not evidence that a Nix build is complete.
- Differential tests must feed the Rust implementation and Valgrind reference
  tools the exact same raw profile. Do not use absolute event totals as golden
  values across different x86_64 hosts.
- The integration matrix must continue to validate 12 raw Callgrind profiles
  and 12 reference annotations unless an intentional test-design change is
  documented in `NOTES.md`.
- Do not commit `target/`, `result`, `results/`, generated Callgrind profiles,
  or other local build output.
- Update `NOTES.md` when a decision is made, a new papercut is discovered, or
  an unresolved design question materially changes.

## Current state

All five workspace members compile, but they are scaffolding. The parser has
only a minimal header-reading function and one unit test. Read `NOTES.md`
before choosing the next implementation slice.
