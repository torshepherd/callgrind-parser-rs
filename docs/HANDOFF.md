# New-session handoff: Callgrind parser and SQLite comparisons

## Reconstruction completed 2026-09-15

The durable harness is now implemented. Start with
[tests/reference/README.md](../tests/reference/README.md) for commands and
current results: 12 native profiles/annotations validated, 186,876 same-file
aggregate rows matched, eight focused fixtures and 17 Python tests passed.
The Rust gate remains 98 passing tests. Nix wiring exists but is unverified
because Nix is unavailable. The user has authorized the annotate port next.
The original reconstruction contract below remains useful historical context;
its statements about missing implementations are superseded by this update.

Updated 2026-09-14. Repository: `torshepherd/callgrind-parser-rs`.
Default branch at handoff: **master**, not main. Resolve it again before editing.

## Read this first

Resume the SQLite/reference **test harness**, not a supposedly finished Rust
annotator. The Rust `callgrind-annotate`, pprof converter, terminal frontend and
web frontend are still stubs. The implemented component is the shared Rust
parser. The annotations mentioned below came from Valgrind's upstream Perl
`callgrind_annotate`, not our Rust binary.

The user is working in ChatGPT Work and cannot manually reconnect or repair
the coding environment. Do not require a previous VM, a local CLI session, or
files from this conversation. Retrieve the repository through the authorized
GitHub connection when shell Git authentication is unavailable. Do not extract
connector credentials or bypass permission failures.

The previous workspace and uncommitted harness were no longer present when
this handoff was written. **The experiment results survived as documentation;
the adapter implementations and generated profiles did not survive in Git.**
Reconstruct and rerun them. Do not report them as checked-in tests or assume
the old absolute paths will work. This handoff commit changes documentation
only; it does not complete the implementation or add a new test pass.

Read in order:

1. [AGENTS.md](../AGENTS.md): setup, invariants, required gates and safe commits.
2. This document: current status and reconstruction contract.
3. [SQLITE-COMPARISON-CHECKPOINT.md](SQLITE-COMPARISON-CHECKPOINT.md): observed
   counts, comparison limits, pinned native build details and papercuts.
4. [SOURCE-AUDIT.md](SOURCE-AUDIT.md) and
   [source-audit-sources.json](source-audit-sources.json): producer/reader
   semantics and pinned source hashes.
5. [TODO.md](../TODO.md) and [NOTES.md](../NOTES.md): remaining work and history.

Historical statements that KCachegrind was only source-inspected describe the
earlier audit. The subsequent native runtime experiment is recorded below and
in the checkpoint; its durable harness remains unfinished.

## What is actually committed and verified

| State | Evidence / boundary |
| --- | --- |
| Incremental parser and interned owned model | `2f6628c261d77555a3f15751919706117fd1a2ca`; subsequently amended by the audit fix |
| Completed Callgrind/KCachegrind source audit | `e928b4dccf6fba7f7c9cefcbb03f60950210e0c1`; combined-part pid/command inheritance fixed; comparison example handles row reordering |
| Last complete Rust gate | At `e928b4d`: formatting, Clippy, nextest and Cargo tests passed, 98 tests total including 95 parser tests |
| Native SQLite experiment | All 12 profiles parsed; every exclusive event sum matched declared totals; all 12 upstream annotations generated |
| Native KCachegrind comparison | 186,924 aggregate rows matched on identical input bytes: 10,008 function-self, 20,394 call-edge, 156,510 source-line-self, 12 totals |
| Durable experiment checkpoint | Documentation-only `42de558feec0064512ff3564a11b81fedf34eba3`, the remote parent inspected for this handoff |
| Not completed | Durable harness, new edge-fixture runs, comparator negative tests, final Rust gates after harness additions, Nix integration checks, all frontend implementations |

The native experiment used pinned source versions but **not the hermetic Nix
closure**. Nix was absent. Do not turn historical row counts or instruction
totals into cross-host golden values. Correctness compares consumers of the
same raw file, not independently profiled executions.

## Immediate work, in order

1. Obtain current repository state and read its instructions. Preserve current
   remote history and any concurrent work. Run `bash .codex/setup.sh` from the
   repository root; use `./scripts/cargo.sh` for subsequent tool shells.
2. Confirm the files below are still missing before reconstructing them. Start
   with the ordinary, cycle-disabled comparison that previously worked.
3. Add deterministic comparator self-tests and minimal edge fixtures. Separate
   expected semantic agreement from explicit reference-reader differences.
4. Regenerate the unchanged 6-by-2 SQLite matrix. Validate exact case sets,
   all 12 raw profiles, all 12 annotations and the 12 manifest entries. Run
   production-parser self/totals checks and same-file KCachegrind comparisons.
5. Integrate the production-parser gate into the Nix smoke derivation. Declare
   dependencies explicitly. Run Nix checks only in a suitable environment and
   report that gate separately from native and Rust results.
6. Run the complete applicable gates, document reproducible commands in
   `tests/reference/README.md`, update NOTES/TODO, and commit working slices
   promptly. Never force-push or replace history with an unborn local snapshot.

Finish this harness before expanding into shared analysis or frontend work.
The audit already establishes the next analysis contract; no parser rewrite
or new annotation implementation is required to complete this test task.

## Missing implementation inventory

These paths existed only in the lost workspace. Their status is historical,
not a statement that the files are available now.

| Intended path | Last observed state |
| --- | --- |
| `crates/callgrind-parser/examples/reference_export.rs` | Compiled and used for all 12 comparisons |
| `tests/reference/kcachegrind-export.cpp` | Initial adapter worked; later optional `--cycles` rebuild had no observed completion |
| `tests/reference/build-kcachegrind.sh` | Built pinned, clean libcore with Qt6Core |
| `tests/reference/compare.py` | Passed syntax compilation and all 12 comparisons |
| `tests/reference/check_matrix.py` | Draft only; not executed |
| `tests/reference/fixtures/` | Six draft fixtures below; not run |
| `flake.nix` changes | Draft example installation and parser-gate wiring; not tested |
| `tests/reference/README.md` | Still needed; no draft was written |

## Adapter reconstruction contract

This is a **test boundary**, not a new production parser or public data model.
Keep all grammar handling in the existing `Decoder<BufRead>` / `parse_reader`.
Function identity is the tuple `(object, defining file, function name)`;
inline attribution is a separate source-file field.

Use line-oriented TSV. Encode every string as hexadecimal UTF-8; encode event
vectors and event-name lists with commas. Counters are decimal integers, never
floating-point JSON values. Each part gets a stable input-order index.

| Tag | Fields after the tag |
| --- | --- |
| P | part index, comma-separated hex event names |
| T | part index, comma-separated exclusive totals |
| F | part index, object hex, defining-file hex, function-name hex, self costs |
| I | same fields as F, but reference inclusive-function costs; diagnostic only |
| E | part index, three caller identity fields, three callee identity fields, summed call count, inclusive edge costs |
| L | part index, three function identity fields, attribution-file hex, line number, self costs |

Rust exporter:

- Parse through the production API and aggregate in ordered maps with checked
  `u128` additions. Validate event-vector widths and symbol/function lookups.
- Sum only self rows for totals; compare against any declared totals. Keep
  call counts and already-inclusive edge costs separate. Never multiply edge
  cost by call count or count it again as self cost.
- Aggregate calls by function pair for this comparison. Preserve a zero-count
  edge with nonzero cost. Aggregate source rows by function, attribution file
  and line; omit line zero only at this reference boundary.
- Normalize absent names and literal `???` to empty strings only in adapters
  to match KCachegrind. Do not change the parser's distinct representations.
- Preserve jumps in the parser; exact jump and call-target-position parity
  are outside this adapter's comparison scope.

KCachegrind exporter:

- Pin `764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14`; verify Git HEAD and an
  unmodified `libcore`. Do not patch the reference to manufacture agreement.
- Build a `QCoreApplication`, initialize `Loader::initLoaders()`, and disable
  cycles with `GlobalConfig::setShowCycles(false)` for ordinary parity.
- Attach a logger that collects diagnostics; treat loader errors or zero parts
  as failure. A tolerant reference loader's process exit alone is insufficient.
- Use `QFile` and `TraceData::load(&input, filename)` for the exact file.
  Avoid the filename-only overload's prefix-based sibling discovery.
- Activate each `TracePart` individually. Export real event names, part costs,
  `functionMap()` self and inclusive costs, `callings(true)` / `called(true)`
  edges and counts, and `sourceFiles()` / `lineMap()` line costs. Inspect the
  pinned APIs while reconstructing; retain integer precision throughout.
- The previous headless build used C++17, `-O2 -fPIC`, Qt6Core only, and these
  16 libcore translation units: `context`, `costitem`, `eventtype`, `subcost`,
  `addr`, `tracedata`, `loader`, `cachegrindloader`, `fixcost`, `pool`,
  `coverage`, `stackbrowser`, `utils`, `logger`, `config`, `globalconfig`
  (each `.cpp`). No GUI or moc step was needed.
- Support normal Qt6Core discovery or an explicit extracted Qt prefix.
  The checkpoint documents the successful native package versions and hashes.

Comparator:

- Parse strictly: total field counts P=3, T=3, F/I=6, E=10, L=8. Reject
  malformed records, invalid integers/hex, duplicate keys and inconsistent
  vector widths. Compare part sets and event-name layouts explicitly.
- Align cost vectors by event name, not column position. Account explicitly
  for KCachegrind's dataset-wide event layout versus part-local Rust layouts;
  a union layout may zero-fill unmeasured columns at this test boundary only.
- Drop zero-only materialized F/L rows and zero-count/zero-cost E rows, but
  retain totals and every edge with either nonzero count or nonzero cost.
- Exclude I from ordinary equality. Compare unique normalized tuple keys and
  values, report useful counts and the first few mismatches, and return
  nonzero for mismatches or exporter failures.
- Test at least a deliberately altered counter, malformed TSV, duplicate
  rows, reordered events and zero-count/nonzero-cost edges. Negative tests
  must demonstrate failure, not just exercise successful examples.

## Focused reference fixtures still to finish

The two fixtures already committed under
`crates/callgrind-parser/tests/fixtures/` are `zero-call-count.callgrind` and
`recursive-cost.callgrind`. Their raw quantities matched in the native
experiment. The recursive fixture has main self 2 and recur self 7;
KCachegrind showed recur inclusive 7, whereas Perl inclusive annotation showed
12. Keep those policies distinct.

Recreate these additional minimal fixtures and verify each expectation:

| Fixture | Intended assertion; new runtime test remains pending |
| --- | --- |
| Inline source | `fi`/`fe` change attribution without splitting function identity; compare self/line/edge costs |
| Event remapping | Two parts with `Ir Dr` then `Dr Ir Dw`; compare by name, never by raw column index |
| Combined aliases | Define compressed file/function names in part 1 and reuse in part 2; Rust preserves file-scoped aliases, pinned KCachegrind resets them |
| Basename collision | `/one/lib.so` + `/one/main.c` and `/two/lib.so` + `/two/main.c`, both `work`; Rust keeps two identities, reference key can merge them |
| Cycle | main self 2, A self 3, B self 4; edges main-to-A cost 7, A-to-B cost 5, B-to-A cost 1, each count 1; raw totals 9, cycle presentation separately scoped |
| Position range | Self row `1:3 4` with `events: Ir`; Rust rejects the unsupported range, reference accepts its range dialect |

Assert the exact relevant diagnostic or semantic difference. Do not skip a
whole fixture whenever the reference fails. The lost optional cycle extension
proposed a C row with part, member names, cycle self and inclusive costs; it
was not validated and is not an established wire contract.

## Matrix execution and gates

The existing [run-matrix.sh](../scripts/run-matrix.sh) is the source of truth
for six configurations, each with SQLite page caches 64 and 4096:
`ir-only`, `branches-and-jumps`, `balanced-cache`, `constrained-cache`,
`roomy-cache`, `full-simulation`. Do not shrink or silently rename the matrix.

With the pinned native tools available on PATH, use a new output directory
under `.dev/reference/` and run from the repository root:

```bash
export LC_ALL=C
export TZ=UTC
mkdir -p .dev/reference
matrix_dir=$(mktemp -d "$PWD/.dev/reference/sqlite-matrix.XXXXXX")
sqlite3 "$matrix_dir/fixture.db" < workload/fixture.sql
test "$(sqlite3 "$matrix_dir/fixture.db" 'PRAGMA integrity_check;')" = ok
bash scripts/run-matrix.sh \
  --fixture "$matrix_dir/fixture.db" \
  --workload workload/workload.sql.in \
  --out "$matrix_dir"
./scripts/cargo.sh run -p callgrind-parser --example inspect --locked --offline -- \
  "$matrix_dir"/*.callgrind
```

`sqlite3` must resolve to the actual
binary or a symlink, not an exec-ing shell wrapper: the default profiling
invocation did not follow the wrapper's new process image. The checkpoint
also records Qt runtime-library lookup, archive ownership and Perl warnings.
Stop on permission failures; Nix/system installation is not required for the
ordinary Rust suite.

The missing `check_matrix.py` should validate exact profile/annotation names,
manifest case pairs, `PROGRAM TOTALS`, and invoke Rust inspect for every file.
Optional Rust/KCachegrind exporter arguments must be supplied together. Keep
the comparator invocation and build commands documented once implemented.

The untested Nix draft added `--bins --examples` to workspace build flags,
installed `inspect` and `reference_export` under `$out/libexec`, and added
Python plus the matrix validator to `smoke`. It used the cargo install hook's
`$tmpDir/examples/` artifacts. Recheck the pinned hook and actually build this;
that draft was not committed or validated. Keep Qt/reference compilation
explicit and separate from ordinary Cargo dependencies.

Required Rust gates after code changes:

```bash
./scripts/cargo.sh fmt --all --check
./scripts/cargo.sh clippy --workspace --all-targets --all-features --locked --offline -- -D warnings
./scripts/cargo.sh nextest run --workspace --locked --offline
./scripts/cargo.sh test --workspace --locked --offline
```

When Nix is available, also run `./scripts/nix.sh build .#smoke -L` and
`./scripts/nix.sh flake check -L`. Report Rust, native-reference and hermetic
Nix results separately. Preserve source/lockfiles and small deterministic
fixtures in Git; do not commit toolchains, generated profiles or build output.

The next handoff should identify its commit, exact commands and observed
results, remaining gaps, and whether the harness is now reproducible from
that commit without this conversation.
