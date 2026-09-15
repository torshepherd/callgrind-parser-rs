# Fresh-session handoff: CI/Nix, then pprof

Updated 2026-09-15. Repository: `torshepherd/callgrind-parser-rs`.
Default branch verified for this update: **master**. Resolve it again when resuming.

## Start here

The parser, durable reference harness and plain-text annotator are implemented.
**Do not reconstruct the harness or treat the annotator as a stub.**
The next sequence is:

1. Add native CI and get the existing Nix checks passing. Fast Rust/Python CI
   is verified green; the user requested fast tests first on 2026-09-15.
2. Implement the first exclusive-cost `callgrind2pprof` converter.
3. Build shared analysis for `textgrind` and `webgrind`.

Read [AGENTS.md](../AGENTS.md) first, then this document. Implementation details
and commands live in [tests/reference/README.md](../tests/reference/README.md),
[the annotator guide](../crates/callgrind-annotate/README.md), and
[SOURCE-AUDIT.md](SOURCE-AUDIT.md). [TODO.md](../TODO.md) is the ordered checklist;
[NOTES.md](../NOTES.md) retains decisions and papercuts. The older
[SQLite checkpoint](SQLITE-COMPARISON-CHECKPOINT.md) is historical evidence,
not the current missing-work list.

## Committed baseline and observed validation

| Item | Current evidence |
| --- | --- |
| Durable reference harness | `21df9e979464b07785f95553210eb1ad939a7510` |
| Implemented annotator | `d0791129da7c606789193310c66ff4d1978b09f1`, confirmed on remote master |
| Rust gates | Formatting, Clippy, nextest and Cargo tests passed; 136 tests |
| Python checks | 21 comparator/matrix tests passed |
| Native SQLite matrix | All 12 profiles, 12 Perl annotations and 12 manifest entries validated; all self totals match |
| Raw Rust/KCachegrind comparison | 186,876 aggregate rows: T=12, F=9,996, E=20,382, L=156,486 |
| Focused native reference fixtures | Eight passed their scoped agreement or exact expected-difference checks |
| Annotator differential | 87 same-file comparisons passed; 35,080 nonzero function rows, plus totals and call trees |
| Nix | Wiring exists; smoke build and flake check have **not** been run in this environment |
| Fast CI | `e295555053617ad715aee2879475a378154131ed`: [push run #1](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/34962686723) completed successfully; every step green |
| Native CI | Workload registry and SQLite integration job added; first actual Nix/Actions run pending |
| Other frontends | `callgrind2pprof`, `textgrind`, `webgrind` remain stubs |

Rust/Python gates were rerun locally from a clean bootstrap and passed, then
the fast workflow completed successfully on GitHub's Ubuntu 24.04 runner.
The native rows above remain prior-session evidence; native/Nix checks were
not rerun in the fast-CI slice. Do not use native counts or instruction totals
as cross-host goldens. Compare consumers of identical bytes.

The parser is established for its documented supported dialect; it is not
complete for every legacy/extended dialect. The audit lists explicit rejections.
A green annotator differential is scoped semantic parity, not byte-for-byte
output or full source-annotation parity.

## Work Mode recovery and publication

The user works in ChatGPT Work and cannot supply a local CLI or repair an old
VM. Assume neither the checkout nor `.dev/`, generated profiles, processes,
or toolchains survive. Start from Git, not old scratch paths.

- Use shell Git when authenticated; otherwise retrieve files through the
  authorized GitHub connection. Do not extract connector credentials.
- Preserve Git file modes when materializing. This session lost executable bits
  during an environment transition; the user authorized restoring the repository
  scripts' recorded permissions and restarting setup. Do not confuse that
  mechanical repair with bypassing an actual access-control failure.
- Run `bash .codex/setup.sh` from the repository root. Use
  `./scripts/cargo.sh` in subsequent tool shells. Bootstrap installs pinned
  Rust/Clippy/rustfmt/nextest and fetches the lockfile; Nix is separate.
- Preserve remote history and concurrent work. The previous local Git directory
  was an inspection snapshot, not a clone containing remote ancestry. Publication
  used the actual remote parent/tree and a non-forced branch update. Never push
  an unrelated snapshot root or force-update the branch.
- Commit useful working slices promptly; generated outputs belong in ignored
  scratch/build directories. Record the actual commit and gate results.

The last interruption also made the execution server unavailable. GitHub API
access still worked and completed the annotator push. A missing execution
server is not evidence that remote work was lost. Do not claim a new test pass
unless a new test actually ran.

## Next task 1: GitHub Actions and Nix

**Current integration implementation (2026-09-15):** user authorized adding
SQLite now and making workloads extensible. `workload/default.nix` is the
registry; `workload/README.md` documents the plan schema and future Clang shape.
The shared runner preserves SQLite's 12 cases. Nix builds a reusable raw corpus,
then the integration app and smoke derivation both check the exact same bytes.
CI keeps the corpus before compiling Rust and saves both annotators' reports
and logs in writable outputs, including comparison failures. The new job and
complete Nix wiring still need an actual green run before marking them verified.
Local Rust gates and 28 Python tests passed; Nix is absent in this Work VM.
No production Rust behavior or software lockfile pins changed in this slice.


**Fast slice complete:** workflow commit `e295555053617ad715aee2879475a378154131ed`
triggered [run 34962686723](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/34962686723)
on a direct push to `master`. The run completed successfully with bootstrap,
formatting, Clippy, nextest, Cargo tests and Python comparator tests all green.
Native integration is still pending. The user accepts Python for now and
prefers eventually porting the comparison harness/tests to Rust; see TODO.md.

The user explicitly narrowed the first slice to fast tests. Keep direct pushes
to the verified default branch (`master`); PRs are optional. The fast workflow
also accepts `main` if renamed later, optional PRs and `workflow_dispatch`.
Inspect CI after pushing; fix a failure or add a revert commit, never force-push
a rollback. No branch-protection changes or automatic rollback are required.

The fast workflow uses the repository bootstrap without caching, declares host
build tools/Python, and pins checkout v7.0.1 by its verified full commit SHA.
Preserve the observed run evidence above when updating CI status.
Native integration is the next separate slice, using the existing harness.

Keep two clearly separated checks:

**Fast Rust/Python job:** on x86_64 Linux, run the repository bootstrap, then:

```bash
./scripts/cargo.sh fmt --all --check
./scripts/cargo.sh clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
./scripts/cargo.sh nextest run --workspace --locked --offline
./scripts/cargo.sh test --workspace --locked --offline
python3 -m unittest discover -s tests/reference -v
```

Declare Python and host build tools explicitly. Cache only as an optimization;
a fresh checkout/cache miss must work. Keep the no-protoc/no-system-zlib
ordinary Cargo build. Select and verify appropriate current Actions when
implementing the workflow; do not silently replace repository lockfile pins.

**Nix integration job:** use a suitable x86_64 Linux runner with flakes enabled.
Run the existing entry points and inspect real failures:

```bash
./scripts/nix.sh build .#smoke -L
./scripts/nix.sh flake check -L
```

The current flake builds the workspace with binaries/examples, installs
`inspect` and `reference_export` under `libexec`, generates the SQLite matrix,
runs Python tests, validates all profiles using production `inspect`, then
runs Rust/Perl annotation comparisons. These are the first execution of this
wiring, not just another validation pass.

Concrete things to check if it fails:

- Nix's Rust toolchain must compile the current workspace and Cargo.lock.
  The native bootstrap toolchain and nixpkgs Rust are separate selections.
- Verify the installed example paths and `$tmpDir/examples/` hook assumption
  against the pinned nixpkgs hook. Source inspection alone was the prior evidence.
- Python scripts must have their sibling imports available in the store;
  the recursive fixture path is supplied explicitly to `check_annotate.py`.
- Declare every native tool/runtime dependency rather than relying on ambient
  Work VM packages. Preserve the six configurations times two SQLite cache sizes.
- The profile-producing SQLite executable must be an actual binary/symlink,
  not an exec-ing wrapper. Keep `LC_ALL=C` and `TZ=UTC`.
- Keep generated profiles, annotations and useful logs as CI artifacts,
  especially on failures. Do not commit them or replace raw counters with
  cross-host constants.
- Nix smoke currently checks Rust parsing and Rust/Perl annotation. It does
  **not** compile/run KCachegrind libcore. The Qt/C++ harness is a separately
  documented native gate. If added to CI, pin and declare it explicitly; do not
  describe a green smoke job as a KCachegrind CI pass.

Trigger checks on pushes/PRs and expose a manual run if useful. Use normal
repository-scoped workflow permissions. If writing workflow files is rejected
by the connector, report that exact capability limitation and preserve a
reviewable draft; do not extract credentials or bypass the rejection.

**Fast slice done means:** the workflow is committed on the default branch,
its run for the relevant commit has completed successfully, and NOTES/HANDOFF
record the run/commit and scope. Native integration can remain deferred for
this slice. **Full CI/Nix task done means:** both jobs have completed green.
Creating YAML or dispatching a run alone is not a completed gate. Retrieve
Actions status/logs through the GitHub connection if shell access disappears.
For push runs use the repository workflow-runs endpoint with `head_sha` and
`event=push`; the connector's commit-workflow-runs helper currently filters to
PR events and can miss the direct-push run. Record integration distinctly from
Rust results; do not mark an unchecked gate complete.

## Next task 2: exclusive-cost pprof converter

The crate already has `callgrind-parser`, `clap`, `prost`, and pure-Rust-backend
`flate2` dependencies. Its one test round-trips a custom one-field
`SmokeMessage` through gzip; **it is not the pprof schema or conversion**.

Start with [SOURCE-AUDIT.md, section 6](SOURCE-AUDIT.md). Recommended first scope:

1. Read through the production parser, choose one part explicitly (reuse the
   annotator's user-facing zero-based part convention), and convert exclusive
   self rows. Aggregate call edges do not establish full sampled stacks.
   Use a single-location representation with documented lost call-path detail;
   exclude inclusive edges from sample values.
2. Define event names/units and source/instruction-location identity before
   coding. Preserve object qualification and attributed source locations;
   do not fabricate inline chains, runtime mappings, load addresses or build IDs.
   Callgrind instruction addresses are not automatically runtime virtual addresses.
3. Use the actual upstream pprof schema at an identified revision with appropriate
   attribution; choose checked-in generated Prost bindings or a suitable pinned
   schema crate. Preserve ordinary builds without requiring protoc.
4. Aggregate with checked unsigned arithmetic, then validate signed protobuf
   ranges before conversion. Check location IDs, string-table indexes and sample
   widths against the chosen schema. Do not silently wrap, clamp or drop costs.
5. Produce deterministic gzip/protobuf output for reproducible tests. Finalize
   the encoder and propagate output errors. Document CLI and unsupported cases
   in the converter README.
6. Add focused tests for repeated locations, unknown names, object/path
   collisions, line zero, instruction-only profiles, different event layouts,
   zero counts, exact exclusive conservation, malformed input and signed-range
   overflow. Multipart input must not be silently merged.
7. Validate generated profiles with an independent pprof implementation, ideally
   a pinned Go google/pprof reader/validator plus a report invocation. This is a
   separate integration dependency, not a reason to require Go for ordinary
   Cargo builds. A decode with our own Prost types is insufficient by itself.
8. Convert the same SQLite raw profiles and compare per-event sample sums to
   production-parser self totals. Declared summary can exceed visible self
   costs; never substitute it as the conservation target.

These are the proposed first implementation contract, not an already-written
converter. Units for unusual events (notably sysTime), unknown positions,
metadata/provenance and any schema-binding dependency need explicit decisions
in the converter docs. Verify the upstream schema/validator while implementing.
Any later full-stack allocation is a separately named approximation.

**Done means:** documented working CLI, focused Rust tests, independent pprof
validation, same-input exclusive-cost conservation, required Rust gates, and a
commit preserving history. Record native/CI/Nix coverage separately.

## Later work and established boundaries

Shared UI analysis comes after those two deliverables: explicit selection/import
provenance, event-name remapping, checked aggregation, source/instruction indexes,
reverse edges and exact SCCs. Keep raw self, call and jump quantities separate.
Do not make either UI depend on the annotator's presentation policy. Benchmark
before replacing parser storage with a new arena or graph layout.

Annotator output stays barebones, per the user's request: aligned text, no
colors, boxes or TUI. Default grouping preserves defining-file function identity;
`--grouping=source` is the explicit Perl-style inline split. Zero-count costs
and calculated self totals deliberately correct Perl limitations. Its parity
script normalizes documented display differences and runs away from source
prefixes to avoid Perl's inconsistent cwd trimming.

Reference sources, build commands and the eight exact fixture expectations are
already in Git. Keep KCachegrind unmodified at the pinned revision; pass an
unopened QFile to its loader, disable cycles for raw parity, and check diagnostics.
Extracted Qt prefixes need transitive runtime libraries as well as headers.

Optional annotate extensions (derived events, full source-output goldens,
multipart aggregation, cycle-aware views) can wait. They do not need to block
the first pprof converter or trigger a parser rewrite.
