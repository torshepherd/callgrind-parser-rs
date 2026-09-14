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
- Parser suite: `./scripts/cargo.sh nextest run -p callgrind-parser --locked --offline`
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
- Cross-check ambiguities against the producer and reader sources. The manual's
  jump grammar differs from Valgrind output; see `NOTES.md`. Do not preserve a
  faulty test contract solely because an earlier agent wrote it.
- `Decoder<BufRead>` owns the one grammar/state machine. `parse_reader` and
  `parse_profile` collect its events into an owned profile. Keep the streaming
  path free of retained cost records and input-wide string buffers.
- Interned string IDs and function IDs are profile-local. Function identity
  includes object, defining file, and name; inline source files are locations.
  Preserve self costs separately from inclusive call costs and jump counts.
- Do not synthesize full stacks or binary/inline metadata absent from the input.
  Pprof signed-range checks and any approximation belong in the converter.
- Read `docs/SOURCE-AUDIT.md` before implementing analysis/frontend semantics.
  A zero call count can accompany nonzero inclusive cost after a dump. Preserve
  these edges. Do not treat reference annotator output as a universal oracle,
  or bake KCachegrind's cycle/inclusive display heuristics into raw parsing.
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

The parser has an incremental decoder, an owned interned model, and active
conformance, property, and streaming tests. Frontend binaries remain stubs.
Read `TODO.md` for the ordered work list and `NOTES.md` for design, compatibility
limits, and validation evidence. The Callgrind/KCachegrind source audit is in
`docs/SOURCE-AUDIT.md`; the 12-profile parsing gate and shared analysis are next.
Do not claim complete format/frontend conformance from passing unit tests.

For externally generated files, run
`./scripts/cargo.sh run -p callgrind-parser --example inspect --locked --offline -- PROFILE`.
This validates declared totals against summed self costs. The `compare` example
compares semantic records across deterministic producer compression settings.
