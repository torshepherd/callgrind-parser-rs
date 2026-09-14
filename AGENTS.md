# Agent instructions

This repository is a Rust parser and analysis-tool suite for existing
Callgrind-format data. It does not implement Valgrind, profiling, or dynamic
binary instrumentation.

## Start here

On a fresh ChatGPT/Codex Linux environment, run:

```console
bash .codex/setup.sh
```

This installs pinned Rust, rustfmt, Clippy, and nextest into `.dev/`, then runs
`cargo fetch --locked`. No root, Nix, Docker, system users, or services are
required. Initial downloads still require permitted network access. Stop and
report permission failures; do not claim tests passed without running them.

Use the wrapper in separate tool shells, or activate plain Cargo in Bash:

```console
./scripts/cargo.sh test --workspace --locked --offline
# Alternatively, in each fresh Bash shell:
source scripts/dev-env.sh
cargo test --workspace --locked --offline
```

Do not assume the VM, tool installation, or checkout survives a conversation.
Resolve the remote default branch before editing. Read/retrieve the repository
through the authorized GitHub connection if shell Git authentication is absent;
do not extract connector credentials. Preserve remote history and concurrent
changes. Never force-push. Commit source, lockfiles, and handoff notes to Git.

Nix is optional for daily Rust work and owns the separate pinned SQLite/
Valgrind integration environment. `.codex/setup.sh` no longer installs Nix.
Use an existing Nix installation or a suitably configured CI runner for it.

## Required commands

Run from the repository root:

- Fast tests: `./scripts/cargo.sh nextest run --workspace --locked --offline`
- Deferred parser contracts: `./scripts/cargo.sh nextest run -p callgrind-parser --locked --offline --run-ignored ignored-only`
- Cargo tests (also covers doctests): `./scripts/cargo.sh test --workspace --locked --offline`
- Formatting: `./scripts/cargo.sh fmt --all --check`
- Lints: `./scripts/cargo.sh clippy --workspace --all-targets --all-features --locked --offline -- -D warnings`
- Callgrind integration matrix: `./scripts/nix.sh build .#smoke -L`
- Complete hermetic check: `./scripts/nix.sh flake check -L`

Before handing off a code change, run formatting, Clippy, nextest, and Cargo
tests. Run relevant integration checks when changing their behavior. If Nix or
native tools are unavailable, report that separately; this must not prevent
running the Rust suite. A green Cargo suite is not a full integration pass.

When changing dependencies, update and commit `Cargo.lock` with Cargo, then run
setup again to prefetch. Keep versions in `[workspace.dependencies]`, enabled
only in consumers that need them. Nextest is a development executable, not a
crate dependency. Preserve a no-`protoc`, no-system-zlib ordinary Cargo build.

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
- Do not commit `.dev/`, `target/`, `result`, `results/`, generated Callgrind profiles,
  or other local build output.
- Update `NOTES.md` when a decision is made, a new papercut is discovered, or
  an unresolved design question materially changes.

## Current state

All five workspace members compile, but they are scaffolding. The parser has a
minimal header reader, a test-fixture builder, and ignored conformance tests
that define successive implementation slices. Read `NOTES.md`, select one
ignored test or tightly related group, and make that slice pass without
weakening its assertions.
