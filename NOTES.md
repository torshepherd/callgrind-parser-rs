# Working notes

This is the living project scratchpad for humans and agents. It is allowed to
be incomplete, repetitive, speculative, and occasionally wrong. Promote stable
user-facing facts to `README.md` and stable execution rules to `AGENTS.md`.

## Current direction

### 2026-09-15: fast CI and direct-push workflow

- User requested fast tests first; the native SQLite/Valgrind/Nix CI job is
  explicitly deferred to the next slice. The parser, annotator and durable
  comparison harness remain implemented; no production-code changes here.
- Added `.github/workflows/ci.yml`: Ubuntu 24.04 x86_64, explicit host tools,
  repository-pinned bootstrap, formatting, Clippy with warnings denied, nextest,
  Cargo tests including doctests, and the existing 21 Python comparator tests.
  Python is existing test-harness code, not a new parser/CLI dependency.
- User accepts the existing Python harness for now but prefers migrating it
  and its tests to Rust eventually. Added a deferred TODO; do not expand this
  fast-CI slice into a harness rewrite or lose its differential coverage.
- Push triggers cover `master` (verified default) and `main` if renamed later;
  optional PR and manual triggers also exist. Direct pushes remain the normal
  workflow. Run local checks, push, inspect the exact commit's CI result, then
  fix or add a revert commit if needed. Never force-push shared history.
- Checkout v7.0.1 is pinned to verified commit
  `3d3c42e5aac5ba805825da76410c181273ba90b1`, with credentials not persisted and
  read-only contents permission. Superseded runs on the same ref are cancelled.
  No cache initially: each run exercises clean setup; cache optimization can
  follow measured need. Existing Rust/nextest/dependency pins are unchanged.
- Fresh local bootstrap succeeded from an empty tool/dependency directory.
  Formatting, Clippy, nextest and Cargo tests all passed (136 Rust tests,
  none ignored); Python unittest passed all 21 tests. Workflow YAML parses
  and `git diff --check` is clean. No native or Nix checks run in this slice.
  First published Actions run is still pending; local results are not CI proof.
- Recovery: shell Git lacks authentication, so all 78 source blobs were read
  through the authorized GitHub connection at `dd1945e5`, verified against
  blob IDs, and materialized with recorded file modes. Local Git is only an
  inspection snapshot. Publication must use remote ancestry and a non-forced
  update. The commit-workflow-runs helper filters to PR runs; use the repository
  runs endpoint with `head_sha` and `event=push` for the direct-push workflow.

### 2026-09-15: current handoff clarified for a fresh Work session

- Confirmed remote master is `d0791129da7c606789193310c66ff4d1978b09f1`;
  the annotate push completed. No GitHub Actions workflow files exist there.
- The old handoff accumulated mutually stale instructions: its upper section
  said annotate was complete, while lower sections still called it a stub and
  directed reconstruction of missing harness files. Replaced it with one current
  entry point. Historical audit/checkpoint evidence remains in its own docs and
  the prior handoff is retained in Git history.
- Ordered the next work explicitly: CI plus real Nix validation; exclusive-cost
  pprof converter with independent validation; shared UI analysis/frontends.
  Added commands, actual CI completion criteria, Nix packaging assumptions,
  converter schema/units/location/range/conservation decisions and test scope.
- Clarified that Nix smoke currently runs parser/Perl annotation checks, not
  the Qt/KCachegrind harness. The pprof dependency test uses a toy SmokeMessage,
  not profile.proto. These are easy traps for a fresh session.
- Recorded Work-mode recovery constraints: no dependence on old scratch paths
  or a user CLI, preserve Git file modes and remote ancestry, and inspect
  GitHub Actions results via the connection if shell execution disappears.
- This is a documentation-only clarification. No new implementation, Rust/native
  test pass, workflow run, Nix pass or pprof functionality is claimed.


### 2026-09-15: plain-text annotator implemented and validated

- Harness milestone is on remote `master`: `21df9e979464b07785f95553210eb1ad939a7510`.
- Implemented `callgrind-annotate` over the existing production parser, with
  checked u128 aggregates, self/inclusive reports, event show/sort, thresholds,
  percentages, caller/callee trees, source context/search and exact TSV output.
  User clarified rendering should be barebones like upstream: plain aligned
  text, no visual UI. Other frontend crates remain stubs.
- Default grouping retains full defining-file function identity. Added explicit
  `--grouping=source` for the upstream inline attribution split, without changing
  the raw parser. Multipart inputs require `--part INDEX`; no accidental merge.
  Derived-event formula evaluation remains outside this first port.
- Inclusive display follows incoming-edge sums (including recursion), falling
  back to self plus outgoing edges. Zero-count calls are never self costs;
  declaration-free program totals always use self sums. Full policies and
  intentional deviations are in `crates/callgrind-annotate/README.md`.
- Same-file differential: 87 invocations passed across two minimal fixtures
  and all 12 SQLite profiles, comparing 35,080 nonzero function rows plus
  program totals and call trees. Covers default/self/inclusive, full tables,
  event selection, thresholds and tree=both. Python comparisons use source
  grouping and normalize percent/whitespace/dot-zero/object decorations;
  full object identity is covered by the independent raw libcore harness.
- Papercut: Perl strips cwd from fl/fi/fe but not explicit callee filenames,
  creating inconsistent identities if run from a source prefix. Differential
  runs now execute from profile directories outside the source tree, with
  absolute profile paths. Perl also omits object decorations for inline rows;
  these display labels are excluded from scoped annotation parity.
- Full gates: formatting, Clippy, nextest and Cargo tests pass, 136 tests total
  (38 new tests). Python suite passes 21 tests. All eight reference fixtures
  and all 12 parser/libcore comparisons remain green (186,876 rows).
- Nix smoke invokes the Rust annotator comparison too, with explicit parser
  fixture paths and declared Python/native tools. Nix is absent, so build and
  flake checks are still unverified; do not present native results as Nix results.
- Another papercut: Cargo replayed an old Clippy diagnostic despite success
  after an edit; touching the changed example forced a clean successful
  recheck. No diagnostic suppression or dependency changes were used.

### 2026-09-15: harness reconstructed, annotate implementation authorized

- Recovered remote `master` at `11d2be8983984fe08227840c4571100813ad6f9e`
  through the GitHub connection. Shell Git authentication is unavailable.
  Local Git is an inspection snapshot only; publish with the real remote
  parent and a non-forced ref update, never push the snapshot root.
- Reconstructed `reference_export`, strict comparator and negative tests,
  matrix validation, pinned headless KCachegrind build/export, and all six
  missing fixtures. Commands and scope: `tests/reference/README.md`.
- Rebuilt pinned Valgrind/SQLite natively and ran the unchanged 12-case matrix.
  All self totals match; KCachegrind comparison T=12, F=9,996, E=20,382,
  L=156,486, total 186,876. All eight fixture expectations pass. Host totals
  and row counts differ from the prior experiment and are not golden values.
- 17 Python tests and all 98 Rust tests pass; formatting/Clippy pass.
  Nix smoke now calls the production parser and Python validator; pinned
  cargo-install-hook was inspected. Nix itself is absent: closure unverified.
- Papercuts: an environment transition terminated processes and stripped
  script execute bits; user explicitly authorized restoring them and setup.
  `TraceData::load(QIODevice*, ...)` opens its own input: pre-opening QFile
  caused a misleading load failure. Qt's transitive libb2 requires
  LD_LIBRARY_PATH when using an extracted prefix despite executable RUNPATH.
  The Perl full-simulation annotation warnings at line 1285 persist.
- User explicitly requested implementing/pushing the annotator after handoff
  work. Next: semantic compatibility on single-part inputs, deterministic
  rankings, options, call trees and source annotation; preserve correct raw
  zero-count/identity semantics, document reference exceptions.

New sessions should start with [docs/HANDOFF.md](docs/HANDOFF.md). It records
the committed baseline, observed native results, missing harness code and a
self-contained reconstruction plan after the workspace reset.

The parser foundation and plain-text annotator are implemented with active
tests. Other frontends and shared analysis remain ahead. `TODO.md` is the ordered checklist. The completed
[source audit](docs/SOURCE-AUDIT.md) supports the current model and defines the
analysis/compatibility work. Native SQLite parsing/reference comparisons have
passed; the harness is committed. Validate the Nix wiring on a suitable host next.

The primary reference is the
[Callgrind Format Specification](https://valgrind.org/docs/manual/cl-format.html),
format version 1. Valgrind's `callgrind_annotate` is a scoped behavioral reference
for the annotate frontend, with documented zero-count/recursion limitations.
Real profiles generated by the Nix integration
matrix complement, but do not replace, minimal grammar fixtures.

## Decisions so far

- This is principally a Callgrind parser library plus frontends for analyzing
  Callgrind-format data.
- This is not a Valgrind or Callgrind instrumentation reimplementation.
- Cargo owns the Rust workspace, dependencies, and unit tests.
- `cargo-nextest` is installed by the development bootstrap and is the preferred
  runner; ordinary `cargo test` remains supported and also runs doctests.
- Easy setup in fresh ChatGPT Work Linux sessions is the requirement, not Nix.
  A repository-local rustup toolchain owns daily Rust development. Nix is an
  optional separate environment for native dependencies and integration tests.
- `nix flake check -L` is the eventual GitHub Actions contract.
- The parser is shared infrastructure. Frontends must consume it rather than
  parse the text format themselves.
- Both streaming and owned use cases use one decoder. `Decoder<R: BufRead>`
  incrementally emits headers, records, and part endings; `parse_reader` and
  `parse_profile` collect those events into owned semantic data.
- Differential tests compare two consumers of the same generated profile.
  Cross-machine raw cost totals are not golden values.
- SQLite is the first realistic profiled workload. More third-party workloads
  can be pinned in Nix when they add distinct coverage.

## Parser architecture (2026-09-14)

The old model was provisional and was replaced rather than preserved as an API
constraint. It duplicated context strings per record, omitted jump source
positions, conflated function identity with inline source attribution, and did
not settle streaming or interning. It also treated derived-event formulas as
opaque strings. Those gaps would have affected all four consumers.

The decoder reads incrementally through `BufRead`, framing complete UTF-8 lines
and using Nom 8 for numeric and expression tokens. It emits `PartStart`,
`Record`, and `PartEnd` events. It retains only the current header, parser state,
compression dictionaries, interned symbols/functions, one bounded line buffer,
and its small event queue. This is **not constant-memory parsing**: dictionaries
grow with unique names/functions. It does not retain all records or the full
input. A consumer may stop reading to cancel and may retain dictionaries with
`into_symbols()`; cancellation does not validate unread input.

`parse_reader` collects those exact events into an owned `Profile` for the TUI
and web backend. `parse_profile(&str)` uses the same path. The owned result has
no input lifetime and is Send + Sync. Lasso interns each distinct text once;
freezing discards construction-time lookup maps and shares immutable tables.
`StringId` and `FunctionId` are typed, profile-local handles, not serialized
wire IDs or stable identifiers across different profiles.

Functions are identified by `(object, defining file, name)`. Cost locations
separately carry the actual source file and position columns, so `fi`/`fe` do
not split one function into unrelated nodes. Unknown context remains absent;
literal `???` names are retained. Calls and jumps have explicit source and
target locations. Recursion/cycles are retained as edges, never expanded into
a tree during parsing. Self costs, inclusive call costs, and jump counts are
distinct. Repeated rows remain available for later checked aggregation.

SmallVec keeps the maximum three position columns inline. Costs are dense
boxed arrays in each part's event order, with omitted trailing values zeroed.
This avoids per-row name maps and spare vector capacity, but is not the final
allocation-optimized storage design. No graph library or unsafe custom arena
is selected before the analysis workload is measured. Proptest is test-only.

All raw counts and positions are checked `u64`; relative overflow/underflow
is an error. Summary and totals remain separate. Event formulas are parsed
into coefficient/name terms, but reference resolution, cycle detection, units,
and evaluation remain analysis work. Comments are discarded; record order and
input line numbers are retained. Unknown headers/descriptions are preserved;
unknown body syntax is rejected rather than silently losing data.

| Consumer | Information available | Work still required |
| --- | --- | --- |
| Terminal/browser analysis | Qualified function IDs, objects/files, source/instruction positions, call/jump edges, event layouts, part/thread metadata | Shared indexes, checked aggregation, cycles, query API and presentation |
| Annotate | Self costs, inclusive edge costs, call counts, source files/lines, summary and totals | Reference-compatible ranking, percentages, recursion and formatting |
| pprof | Function/location names, available addresses/lines, raw events and values, thread metadata | Schema, checked u64-to-i64 conversion, units, mapping policy, gzip/Prost output and independent validation |

Pprof samples describe stacks; an aggregate Callgrind graph does **not generally
determine the original full stacks**. The converter must select and document a
policy (for example exact exclusive leaf samples with reduced call-path detail,
or an explicitly approximate path allocation). It must not count inclusive
edges again as self samples, fabricate binary load ranges/build IDs or inline
chains, or cast overflowing counters to signed protobuf values. String interning
here does not replace pprof's own string table, whose first entry must be empty.

### Compatibility findings from targeted source checks

- The manual's jump description diverges from Valgrind 3.26.0 `dump.c`:
  actual conditional jumps are `jcnd=taken/executed target`, and both jump
  kinds have a following **source-position** row. `jfi`/`jfn` select jump
  destinations. KCachegrind's loader agrees with the producer. The old ignored
  jump fixture had neither the actual counter syntax nor the source row.
- File tags (`fl`, `fi`, `fe`, `cfi`, `cfl`, `jfi`) share a format compression
  namespace; `fn`, `cfn`, `jfn` share another; `ob`, `cob` share the third.
  Sparse wire IDs use hash maps. Redefinitions affect future uses only. Current
  multipart policy retains aliases through the file and resets part-local
  layout, position, and context state. The full audit confirms file-scoped
  aliases and distinguishes combined dumps from separately imported files.
- The decoder accepts version 0 as well as version 1, following the manual's
  compatibility prose. Explicitly empty/duplicate event declarations fail.
- We support actual Valgrind jump syntax, not the ambiguous single-line jump
  dialect described in the manual. Input-wide source compatibility is not yet
  claimed. Compressed mangled contexts are explicitly rejected as unsupported;
  the full audit records their exact producer forms and follow-up scope.

Sources examined: [format manual](https://valgrind.org/docs/manual/cl-format.html),
[Valgrind 3.26.0 source archive](https://sourceware.org/pub/valgrind/valgrind-3.26.0.tar.bz2)
(`callgrind/dump.c`, `callgrind/callgrind_annotate.in`),
[KCachegrind loader](https://github.com/KDE/kcachegrind/blob/master/libcore/cachegrindloader.cpp)
(Git blob `0289e8f68f2ec209dbdac3bb9e64132251e88903`), and
[pprof schema](https://github.com/google/pprof/blob/main/proto/profile.proto).
These were targeted preliminary checks. The completed audit is now in
`docs/SOURCE-AUDIT.md`, with pinned revisions and a file-hash manifest.

## Test plan

### Parser unit fixtures

Current harness status: 95 active parser tests (including fixed-seed property
tests) and three frontend dependency smoke tests. No parser contracts are
ignored. Run the parser suite with:

```console
./scripts/cargo.sh nextest run -p callgrind-parser --locked --offline
```

Add focused regression cases before extending behavior. Earlier ignored tests
were provisional: the stale jump test was corrected against the producer,
rather than made into a passing test of the wrong grammar.

Use small inline strings when a test targets one rule. Build a test-only
profile generator once combinations of positions, events, and associations
would otherwise obscure the assertion. The generator should produce valid
text, not duplicate parser internals or validate its own output.

Coverage slices from the format specification:

- Optional format marker, optional version, optional creator, comments, blank
  lines, and multiple profile parts.
- Part metadata: `pid`, `thread`, `part`, `cmd`, repeated `desc`, `summary`, and
  the final `totals` consistency line.
- Required `events`; default `positions: line`; explicit `instr`, `bb`, and
  `line` position combinations in their defined order.
- `event` long names and inherited event expressions, including coefficients
  and sums.
- Decimal and hexadecimal 64-bit numbers.
- Cost rows with every event present, omitted trailing event costs (which mean
  zero), repeated positions (whose costs sum), and multiple subpositions.
- Absolute subpositions plus relative `+N`, `-N`, and repeated `*` forms, with
  independent history for each subposition column.
- Context specifications: `ob`, `fl`, `fi`, `fe`, and `fn`.
- Called-context specifications: `cob`, `cfi`, historical alias `cfl`, and
  `cfn`.
- Name-compression definitions and references, with object, file, and function
  namespaces shared across their caller/callee/jump aliases.
- Calls: called context, `calls=count target-position`, and its mandatory
  following inclusive-cost row.
- Actual Valgrind `jump=count target` and `jcnd=taken/executed target`, each
  followed by a source-position row; `jfi` and `jfn` destination context.
- Names containing spaces and punctuation where the grammar permits the rest
  of a line to be arbitrary text.
- Useful malformed cases: missing `events`, bad compression references,
  incorrect column counts, overflow, malformed association pairs, invalid
  position ordering, and unsupported format versions.

For each accepted fixture, assert structured meaning rather than merely
`is_ok()`. For each rejected fixture, assert a stable error category and source
location rather than the full presentation string.

### Active coverage and remaining gaps

| Test file | Coverage |
| --- | --- |
| `conformance.rs` | 76 cases: headers/parts, event terms, all position layouts, full u64 range, compression, qualified and inline contexts, calls/recursion/cycles, actual jumps, malformed input and located errors; includes fixture-builder check |
| `streaming.rs` | 8 cases: all small buffer sizes including UTF-8 splits, event ordering, early emission and fused errors, dictionary growth, invalid encoding, I/O failure, line-size limit, owned lifetimes/Send+Sync |
| `properties.rs` | 5 fixed-seed tests × 128 generated cases: decimal/hex equivalence, relative/absolute equivalence, zero-padding, arbitrary-byte/chunk equivalence, arbitrary UTF-8 error handling |
| `source_audit.rs` | 5 cases: combined process metadata, zero-count calls, recursive cost separation, full-path/tuple identities and unsupported source-audited dialects |
| `src/lib.rs` | Legacy header-scanner compatibility for the frontend stubs |

There are no ignored parser tests. This is sufficient to start using and
extending the foundation with regression tests, not evidence of complete
format support. Remaining gates include KCachegrind runtime comparisons, 12-profile
SQLite parsing, allocation/throughput benchmarks, derived-event evaluation,
and independent annotate/pprof conformance. The dependency gzip/Prost test is
still not a pprof conversion test.

### Generated fixture utility

A likely test-only builder shape:

- profile-level marker, version, and creator;
- one or more parts;
- ordered positions and events;
- context/name mapping declarations;
- self-cost records;
- call and jump associations; and
- optional summary/totals.

Keep an escape hatch for inserting literal lines so new grammar cases do not
require expanding the builder first. Do not expose this builder as public API
unless a real user-facing profile-writing use case emerges.

### Reference and integration tests

The existing Nix smoke test produces 12 SQLite profiles from two SQLite page
cache sizes and six Callgrind configurations, plus 12 annotations from the
reference `callgrind_annotate`.

Planned layers:

1. Parse every generated profile and verify structural invariants.
2. Snapshot or semantically compare the Rust annotator with
   `callgrind_annotate` on the exact same profile.
3. Convert profiles with `callgrind2pprof` and validate the resulting protobuf
   with an independent pprof reader/tool.
4. Exercise `textgrind` through model/state tests before terminal rendering.
5. Exercise the `webgrind` backend API independently of the browser UI, then
   add a small end-to-end browser smoke test only when the frontend exists.

## Implementation plan

The model, decoder, and initial active grammar suite are implemented. Follow
`TODO.md` next: full 12-profile parsing checks, shared analysis,
and then frontend behavior with independent references. Passing the current
suite establishes a foundation, not complete compatibility with every producer.

Prefer thin vertical slices: add a focused failing test, implement that rule,
run the fast suite, and periodically prove the whole Nix check.

## Unsolved questions

- Which additional producer dialects should be supported: compressed mangled
  contexts, position ranges, deprecated recursion tags, basic-block detail
  records, and the manual's divergent single-line jump description? Currently
  unsupported body syntax is an error, not silently discarded data.
- Should non-UTF-8 filenames be represented as bytes? Current input is UTF-8
  (ASCII is a subset), with invalid encodings rejected rather than replaced.
- Which configurable upload/record/symbol budgets should the web backend use?
  Individual input lines are capped at 8 MiB now; dictionaries and the owned
  record collection still grow with input size.
- What measured allocation/latency targets should guide compact record storage
  and shared analysis indexes? The initial model has no scaling benchmark yet.
- How should unknown or cyclic derived-event references be diagnosed during
  analysis, and how should overflow in coefficient evaluation be reported?
- How should part/thread selection, merging, and graph cycles be presented?
- How exact should `callgrind-annotate` output compatibility be: byte-for-byte,
  normalized text, or semantic tables with an optional compatibility renderer?
- How should Callgrind events map to pprof sample types and units?
- Does `webgrind` conflict too strongly with the name of the existing Xdebug
  viewer, and should it eventually be renamed?
- What frontend technology keeps `webgrind` reproducible without making the
  Rust parser depend on a large JavaScript toolchain?
- Which additional pinned workloads add meaningful grammar or scaling coverage
  beyond SQLite?

## Papercuts

- A materialized Valgrind source/build snapshot retained its binaries but lost
  the `.in_place` launcher symlinks, so `vg-in-place` initially reported a
  missing tool. Restoring ordinary links to the existing tool/preload files
  fixed it. The generated annotation script also lacked its executable bit;
  invoking it with Perl worked. Neither required extra privileges or rebuilding.
- The native producer orders function contexts using pointer comparisons.
  Different executions can emit equivalent records in different orders;
  semantic compression comparisons must sort within parts and keep duplicates.
- Shell Git could clone public upstream sources but private cloning lacked
  credentials. Use the authorized connector for private reads/commits, with
  the remote parent and non-forced default-branch update. Do not extract tokens.
- The flake currently targets only `x86_64-linux`.
- The Rust Nix derivation copies the whole repository, so documentation-only
  edits invalidate its source hash and rerun the small Rust build.
- `result` can point to either the workspace build or the Callgrind smoke
  output depending on the last `nix build` command.
- The old Nix installer downloaded successfully on 2026-09-13, but failed with
  EPERM while looking up `nixbld`. Do not diagnose every bootstrap failure as a
  network problem. The replacement Rust bootstrap needs no system-user changes.
- Archive extraction under root in this managed VM tried to restore a foreign
  uid/gid and failed. Extract tool archives with `--no-same-owner` and
  `--no-same-permissions`; do not request extra privileges for this.
- The crates.io metadata API returned HTTP 403, while Cargo's standard sparse
  registry and crate download path worked. Metadata lookup failure is not proof
  that `cargo fetch` is blocked.
- `rustup show active-toolchain` implicitly installed missing components but
  warned this behavior is deprecated. Bootstrap explicitly installs missing
  toolchain components instead.
- `.codex/setup.sh` is an ordinary script, not an automatic hook for every Work
  chat. Future agents must obtain the repository, read `AGENTS.md`, and run it.
  Fresh installations need network access; warmed Rust checks run offline.
- The scripted binary bootstrap currently supports only x86_64 Linux. Other
  platforms should use their own rustup and the checked-in toolchain file.
- The local source snapshot initially had an unborn Git branch with every file
  staged, not a clone containing remote history. Its 26 files matched remote
  `b67d697` exactly. Remote updates must use the real remote parent, never turn
  that local snapshot into replacement history.
- Host CPU dispatch can change optimized library code paths even with a pinned
  x86_64 userspace closure. This is why the matrix validates structure instead
  of absolute event totals.
- The legacy `parse_header` helper is only a field scanner retained for the
  frontend stubs. Actual input validation uses the shared decoder.

## Work log

### 2026-09-14: durable new-session handoff

- Confirmed remote `master` at `42de558feec0064512ff3564a11b81fedf34eba3`.
  The old workspace and uncommitted reference harness were no longer present.
- Added [HANDOFF.md](docs/HANDOFF.md) with a fresh-session read order, verified
  versus pending status, adapter reconstruction contract, fixture expectations,
  native matrix commands and required completion gates. A new session must not
  depend on old paths or conversation patches, or ask the user to reconnect a VM.
- This is documentation only. No harness source was recovered or implemented,
  no new tests were run, and the Rust annotator remains a stub. The earlier
  checkpoint preserves historical evidence, not reproducibility from Git.

### 2026-09-14: SQLite/KCachegrind native checks, environment interruption

- All 12 native SQLite profiles parsed and passed self/totals checks. A headless
  adapter against the pinned KCachegrind core matched 186,924 aggregate rows
  across the same inputs (functions, edges, source lines and totals).
- Rust callgrind-annotate remains a stub. These are parser/reference comparisons,
  not output comparisons between complete annotators.
- The coding environment disappeared before the new harness, edge fixtures and
  Nix integration edits could be finished and committed. GitHub remains usable.
- [SQLITE-COMPARISON-CHECKPOINT.md](docs/SQLITE-COMPARISON-CHECKPOINT.md) preserves
  verified evidence, exact scope, native build details and recovery instructions.
  Do not count the pending edge fixtures or full Rust/Nix gates as completed.

### 2026-09-14: Callgrind / KCachegrind source audit

- Completed the planned source review against Valgrind 3.26.0 and KCachegrind
  `764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14`. Findings, precise source map,
  compatibility decisions, native reproduction and follow-up analysis contract
  live in `docs/SOURCE-AUDIT.md`; file hashes live alongside it.
- Verified all 43 local baseline files against remote `2f6628c` before edits.
  The remote default branch is `master`; preserve that existing history.
- Retained the decoder/interned model design. Shared analysis needs per-part
  event remapping, provenance, checked aggregation, raw call-site/reverse
  indexes and exact SCCs; KCachegrind inclusive/cycle-cut conventions belong
  in optional presentation policies. Source/binary lookup stays outside parsing.
- Reproduced and fixed effective process metadata loss after the first combined
  part. Only pid/command inherit; part-local state still resets. Added five
  active source-audit tests and two minimal reference-readable fixtures.
- Demonstrated native `calls=0` with nonzero inclusive costs across dump
  boundaries. `callgrind_annotate` misattributes that cost to caller self;
  it also gives a different recursive inclusive result from KCachegrind's
  inspected policy. Never use its successful exit as universal conformance.
- Ran seven native configurations with `workload/source-audit.c`. Combined
  compressed/uncompressed profiles, ordinary caller/recursion separation and
  basic-block position files passed self/totals checks. Nested mangled names,
  basic-block detail records and unmangled contexts were explicitly rejected.
  Updated `compare` to handle reordered records; 5525 entries matched while
  retaining per-part boundaries and duplicate rows.
- Formatting, Clippy with warnings denied, nextest, and Cargo tests passed
  locked/offline: 98 tests total, including 95 parser tests. Final combined
  profile self/totals checks and compression comparison passed after the fix.
- KCachegrind findings were source-inspected, not runtime-tested; Qt/CMake
  tooling was absent. Nix and the 12-profile SQLite matrix were not rerun.
  Those limits remain explicit next gates in `TODO.md`.

### 2026-09-04

- Reset the old repository contents while preserving the original commit in
  history.
- Added a five-member Cargo workspace with compiling stubs.
- Added a pinned Nix development shell with Rust, nextest, SQLite, and Valgrind.
- Added a deterministic SQLite workload and 12-case Callgrind/reference-
  annotation smoke matrix.
- Proved rustfmt, Clippy, nextest, the Nix workspace build, and the complete
  flake check.
- Added a test-only profile builder, an initial normalized parser model and
  error taxonomy, and 19 ignored grammar contracts spanning valid profiles,
  compression, associations, multi-part input, and malformed input.

### 2026-09-13: portable Rust development and initial dependencies

- Replaced the Nix-only `.codex/setup.sh` with a repository-local bootstrap:
  Rust 1.98.1 (minimal profile, rustfmt and Clippy), rustup 1.29.1, and nextest
  0.9.144. Rustup and nextest download artifacts have checked-in SHA-256 hashes.
  The Rust toolchain is pinned in `rust-toolchain.toml`; rustup verifies its
  component downloads. `.dev/` holds tools/caches and is excluded from Git and
  the Nix source. No shell startup files, system users, or services are changed.
- Added `scripts/cargo.sh` for independent agent shells and
  `source scripts/dev-env.sh` for ordinary Cargo. Initial setup prefetches
  `Cargo.lock`; subsequent tests can use `--locked --offline`.
- Declared Prost (0.14 series) and flate2 (1.1, pure-Rust backend) in
  `callgrind2pprof`, Ratatui (0.30.2) and Crossterm (0.29) in `textgrind`, and
  Clap (4.x, derive) in the four application crates. Cargo.lock pins the full
  112-package external graph. Workspace MSRV is now 1.88 to accommodate the
  TUI dependencies; the development toolchain is pinned separately.
- The parser remains dependency-free. No `prost-build`/`protoc` requirement was
  added: decide schema generation when implementing pprof, keeping ordinary
  builds free of protobuf compiler installation. The gzip test is not a pprof
  conversion test. The web framework and async runtime are still undecided.
- Added isolated dependency smoke tests for Clap derives, protobuf/gzip, and
  Ratatui's in-memory TestBackend plus Crossterm event types. No TTY, external
  processes, network, or native profiler is needed by these tests.
- Replaced Nix's empty-cache `runCommand` Rust build with `buildRustPackage`
  using `cargoLock.lockFile`, workspace build/test flags, and nextest. This
  declares crate downloads before the offline build instead of relying on an
  ambient Cargo cache. The SQLite fixture and all 12 matrix cases are unchanged.
- Verified from a clean source copy with an empty tool directory and Cargo
  cache: bootstrap downloaded and installed its tools and locked dependencies;
  rustfmt, Cargo tests, nextest, Clippy with warnings denied, and a release
  workspace build passed. All build/test commands after setup used
  `--locked --offline`. Cargo and nextest each passed 6 active tests, with 19
  deliberately ignored parser contracts. Warm bootstrap also passed with
  Cargo offline and from outside the checkout directory.
- Formatted the previously unchecked conformance tests without changing their
  assertions. Nix and the native profiling matrix have not been rerun in this
  VM; their prior success does not validate the new Nix Rust derivation.

### 2026-09-14: parser model audit and implementation

- Replaced the old owned-string scaffold with the model/decoder described
  above. Added Nom 8.0.0, Lasso 0.7.3, SmallVec, and test-only Proptest 1.11.0;
  Cargo regenerated the locked dependency graph. Rust setup remains unchanged.
- Replaced 19 ignored contracts with active conformance and failure tests,
  added streaming/lifetime checks and fixed-seed property tests. The workspace
  now has 93 passing tests: 90 parser tests and three dependency smoke tests.
  Formatting, Clippy with warnings denied, Cargo tests, and nextest pass using
  locked, offline Cargo commands. No tests launch Valgrind or other subprocesses.
- Built Valgrind 3.26.0 in `.dev/reference/` using the host toolchain, without
  system installation. Source archive SHA-256:
  `8d54c717029106f1644aadaf802ab9692e53d93dd015cbd19e74190eba616bd7`.
  Ran the checked-in `workload/parser-smoke.c` as a statically linked binary.
  A profile with instruction/line positions, jumps, cache and branch simulation
  parsed into 5,291 records, 108 functions, 185 calls, and 345 jumps. All 13
  self-cost sums matched the producer's `totals:` values. `callgrind_annotate`
  also read that exact profile successfully. Its instruction summary was
  217,155, while emitted self costs/totals were 217,153: another reason to
  preserve both fields. These numbers are observations, not cross-host goldens.
- Two additional producer runs with instruction counters only, jumps enabled,
  and string/position compression respectively enabled and disabled produced
  matching semantic records, including every resolved source/target position,
  function, count, and cost. The `compare` example compared 5,292 entries (one
  part header plus 5,291 records), excluding process metadata and input line
  numbers. Each also passed self-cost/totals checks.
- These are native reference smoke runs, **not** a rerun of the Nix checks or
  the 12-profile SQLite matrix. Nix checks remain unverified in this session.
- Added `TODO.md` with the complete Callgrind/KCachegrind source dive explicitly
  first. The targeted emitter/loader checks above are not that full audit.

Native reference commands, once Valgrind and a C toolchain are available:

```bash
mkdir -p .dev/reference
cc -static -g -O0 -fno-inline workload/parser-smoke.c -o .dev/reference/parser-smoke
for compression in yes no; do
  valgrind --tool=callgrind --error-exitcode=99 \
    --collect-jumps=yes --dump-instr=yes --cache-sim=no --branch-sim=no \
    --compress-strings="$compression" --compress-pos="$compression" \
    --callgrind-out-file=".dev/reference/encoding-$compression.callgrind" \
    --log-file=".dev/reference/encoding-$compression.log" \
    .dev/reference/parser-smoke
done
./scripts/cargo.sh run -p callgrind-parser --example inspect --locked --offline -- \
  .dev/reference/encoding-yes.callgrind .dev/reference/encoding-no.callgrind
./scripts/cargo.sh run -p callgrind-parser --example compare --locked --offline -- \
  .dev/reference/encoding-yes.callgrind .dev/reference/encoding-no.callgrind
```

The static link above was tested on this x86_64 Ubuntu host. It avoids requiring
dynamic-loader debug symbols for the reference smoke run; it is not a portable
native-tool bootstrap. Default Cargo unit/property tests need neither native
reference tools nor these generated files.

References: [rustup installation](https://rust-lang.github.io/rustup/installation/index.html),
[nextest binaries](https://nexte.st/docs/installation/pre-built-binaries/),
[Prost](https://docs.rs/prost/latest/prost/),
[Ratatui](https://docs.rs/ratatui/latest/ratatui/), and
[Nix Cargo.lock vendoring](https://nixos.org/manual/nixpkgs/stable/#importing-a-cargo.lock-file).
