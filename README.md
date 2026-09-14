# callgrind-parser-rs

`callgrind-parser-rs` is a Rust library for reading and analyzing the
[Callgrind profile format](https://valgrind.org/docs/manual/cl-format.html).
The repository also houses a family of frontends built on the same parser:
annotation and conversion tools, a terminal interface, and a web interface.

The parser is the center of the project. The applications exist to make
Callgrind data useful in more places without each tool growing its own partial
and subtly incompatible reader.

> [!NOTE]
> The parser foundation is implemented with incremental decoding, an owned
> interned model, and active grammar/property tests. The applications remain
> stubs. Full producer compatibility and analysis semantics are still in progress.

## Why this exists

Linux `perf` and hardware performance counters are excellent when the host,
kernel, permissions, hypervisor, and PMU virtualization all cooperate. That is
not always the environment where performance work has to happen. Virtual
machines, containers, CI workers, and locked-down systems frequently expose
incomplete counters or no usable counters at all.

Callgrind remains valuable in those environments because Valgrind collects a
portable, detailed software-instrumented profile without depending on access
to the host PMU. It is slow, but it can provide instruction costs, simulated
cache behavior, branch behavior, call relationships, and source attribution
where hardware-assisted profiling is unavailable or unreliable.

The format has outlived much of the tooling around it. KCachegrind is an aging
Qt desktop application with a cumbersome interaction model, especially for
remote, terminal-first, automated, or browser-based workflows. The goal here
is a trustworthy reusable parser plus modern ways to inspect the same data.

## What this is not

This project does **not** reimplement Valgrind or Callgrind's instrumentation.
It does not execute programs under dynamic binary instrumentation, simulate a
processor, or collect profiles. Valgrind remains the reference producer of the
data.

It is also not an argument that Callgrind should replace `perf` on a machine
with good PMU access. Hardware counters and sampling profilers answer important
questions with dramatically less overhead. This project is for consuming the
Callgrind format well when Callgrind is the right producer.

## Workspace

| Package | Purpose |
| --- | --- |
| `callgrind-parser` | The primary library: parse Callgrind files into a shared data model and expose analysis primitives. |
| `callgrind-annotate` | A compatible, scriptable Rust alternative to `callgrind_annotate`. |
| `callgrind2pprof` | Convert Callgrind data for use with pprof-compatible tools. |
| `textgrind` | Explore profiles interactively in a terminal. |
| `webgrind` | Explore profiles through a parser-backed web application. |

## Development

Everyday development needs Rust and Cargo, not Nix, Docker, Valgrind, or SQLite.
For a fresh x86_64 Linux ChatGPT Work session, after obtaining this repository:

```console
# Install the pinned Rust toolchain and nextest; fetch locked dependencies.
bash .codex/setup.sh

# The wrapper works across fresh shells without changing shell startup files.
./scripts/cargo.sh nextest run --workspace --locked --offline
./scripts/cargo.sh test --workspace --locked --offline

# Or use ordinary Cargo after activating the environment in your current Bash.
source scripts/dev-env.sh
cargo test --workspace --locked --offline
```

Setup installs only into the ignored `.dev/` directory, requires no root,
and can be rerun after the VM or caches disappear. Initial setup needs network
access plus Bash, curl, tar, sha256sum, and a working C linker (`cc`). It does
not automatically run merely because a chat opens; the agent runs it after
reading `AGENTS.md`. Git is the durable source of truth, not `.dev/` or `target/`.
Other platforms can use their own rustup installation with `rust-toolchain.toml`.

Nix remains an **optional, separate integration environment**. With Nix
installed, run `./scripts/nix.sh build .#smoke -L` for the profiling matrix or
`./scripts/nix.sh flake check -L` for the Nix Rust build plus that matrix.
The integration smoke test builds a deterministic SQLite fixture, runs a
representative workload through a matrix of Callgrind options, and validates
the resulting profiles with Valgrind's `callgrind_annotate`. See
[SMOKE-TEST.md](SMOKE-TEST.md) for the matrix and manual commands.

Contributors and coding agents should read [AGENTS.md](AGENTS.md) before making
changes. Evolving plans, papercuts, decisions, and open questions live in
[NOTES.md](NOTES.md); the ordered work list is [TODO.md](TODO.md).

## Parser API

`parse_profile(&str)` and `parse_reader(impl BufRead)` return owned profiles
with interned names, qualified function identities, per-part metadata and event
layouts, self costs, calls, and jumps. `Decoder<BufRead>` emits the same semantic
records incrementally for consumers that do not need to retain every row.
The decoder and collector share one parser. Analysis indexes and pprof
conversion are separate consumers of this model.
