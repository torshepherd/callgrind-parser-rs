# Working notes

This is the living project scratchpad for humans and agents. It is allowed to
be incomplete, repetitive, speculative, and occasionally wrong. Promote stable
user-facing facts to `README.md` and stable execution rules to `AGENTS.md`.

## Current direction

### 2026-09-22: first-release preparation

- User authorized publication of the six implemented crates, release-plz
  automation and Linux x86-64/ARM64 plus macOS Intel/Apple Silicon binaries.
  Prepared shared version 0.1.0, registry dependency requirements, repository
  metadata and packaged dual-license files. UI stubs remain unpublished.
- Release-plz 0.3.169 prepares release PRs and publishes only release commits
  after full CI. Cargo-dist 0.33.0 owns binary GitHub Releases. Explicit
  workflow dispatches let the built-in GitHub token start PR validation and
  binary builds; a personal GitHub token/App is unnecessary for this setup.
- Added a tested exact-commit CI gate and four-platform cargo-binstall checks
  with compilation/quickinstall fallback disabled. The release configuration
  and recovery procedure are in docs/RELEASING.md. Automation is held behind
  RELEASES_ENABLED until initial publication and Trusted Publishing are set up.
- Local validation: all six crates passed Cargo's workspace publication dry
  run and packaged-README/license inspection. Formatting, Clippy, 169 nextest
  tests, Cargo tests/doctests, 36 comparator tests, five release-gate tests,
  workspace rustdoc, dist plan and actionlint passed. Native CI, first uploads
  and the four-platform binary workflow still need to run for this slice.

### 2026-09-22: experimental status and documentation provenance

- User requested prominent publication notices: all libraries/tools are
  experimental, project documentation is fully LLM-generated, and the docs
  will receive a review and cleanup pass before 1.0. Added the notice to the
  project README, all six implemented crates' READMEs and their crate-level
  API docs. Added missing parser/writer/pprof library READMEs and explicit
  Cargo `readme` fields so each registry page receives its own notice.
- Rename commit `07c0c89ed3ef84a5db0adfcc2d011fbd3b148a0e` passed all three
  jobs in [push run 35771501753](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35771501753),
  including SQLite smoke/full flake checks and pprof/KCachegrind integration.

### 2026-09-21: frontend, remote workflow, binary tooling and testing discussion

Status: requirements and candidate tradeoffs, **not a finalized stack**. The
user explicitly asked to discuss what matters before fixing decisions that
force particular frameworks or systems. This notes update is authorized; it
does not authorize treating every assistant suggestion below as a user choice.
Both UI crates are still stubs. Shared analysis is still unimplemented; the
existing [inclusive-cost contract](docs/INCLUSIVE-COST-DESIGN.md) remains the
semantic starting point. No code, dependency or benchmark changes in this pass.

#### Confirmed purpose and initial loading scope

The user's core motivation is profiling on an SSH box while investigating
from the local development machine. Two desired form factors:

1. Run webgrind on the remote box and forward its port to a local browser.
2. Run textgrind on the remote box directly over SSH.

Profiles, binaries, debug information and source can therefore remain on the
remote box. A local Mac browser viewing Linux ELF data does not itself require
Mach-O support. Host-platform support and inspected-binary formats/architectures
are separate questions; neither support matrix has been selected yet.

The user proposed Axum for the backend and CLI-supplied paths at startup:
profile, binaries for disassembly, debug information for symbolization, and
source files/directories. This approximates the setup normally done through
KCachegrind's File > Open workflow. Opening files from an already running UI is
explicitly a later goal. Exact flags and path configuration formats are open.

Assistant recommendation: only the profile should be required; binaries, debug
files and source are optional enrichment. Callgrind often already records names
and source locations. Missing enrichment must leave cost/navigation views
useful. Equivalent loading options should serve both commands, including
object-to-local-binary mappings, debug search locations and source-path remaps.
Design the loader so a later UI can reuse it without restarting the process.

A future Open dialog should naturally browse the **server's filesystem** for
this workflow. A browser-native file picker selects/uploads files from the
browser's machine; that is a different feature, not the assumed implementation
of remote Open. Hosted multiuser service design and upload/session management
are not the current target.

#### Proposed structure and consequences of the SSH workflow

- Shared Rust analysis and source/binary resolution beneath both frontends.
  Axum exposes queries for the browser; textgrind calls the Rust code directly,
  without requiring an HTTP server. Responsibility boundaries do not yet fix
  the number of crates or the API's public types.
- Proposed default: webgrind listens on remote loopback, reached by SSH port
  forwarding. Serve all browser assets, including JS, CSS, fonts and icons,
  locally; the investigation should work without a CDN/internet connection.
- Keep the full dataset and expensive analysis on the remote box. Send bounded
  tables, selected graph neighborhoods and source/disassembly windows rather
  than serializing the entire profile into the browser.
- Account for SSH latency: fetch useful view chunks, cache results and avoid
  one network request per row or keystroke. Keep the interface responsive while
  expensive work happens. Exact pagination, caching, cancellation and scheduling
  mechanisms remain open.
- Do not couple either renderer to the other's framework, duplicate parser or
  cost semantics in UI code, or prematurely replace the existing data model.

These are architectural recommendations from the discussion, not implemented
or benchmark-validated guarantees.

#### Performance target: as good as or better than KCachegrind

Asked about typical profile sizes, the user said: "I'd like to do as good or
better than kcachegrind." Do not invent a maximum file size or require the user
to estimate one. Use KCachegrind as the measured baseline.

The discussed comparison plan covers:

- Loading: time until investigation can begin, as well as full load/index time.
- Memory: peak during loading and steady-state use. For webgrind count **both
  server and browser**, rather than hiding cost in the second process.
- Interaction: sorting/filtering, event and part changes, caller/callee
  navigation, and opening source/assembly; continued responsiveness during work.
- Remote behavior: measure the browser through simulated SSH latency, not only
  localhost, so an apparently fast implementation does not fail the core use case.

Compare identical profiles across KCachegrind, webgrind and textgrind, with
comparable operations and explicit measurement conditions. Build a reproducible
corpus varying function count, edges, instruction detail, event columns and
parts; byte size alone is insufficient. Start with generated workloads, add
real user profiles when available, and baseline before setting numeric budgets.
No new performance results or parity claims exist yet. Benchmark current
storage before changing it, and exercise candidate UI components on substantial
datasets before making framework decisions.

#### Frontend candidates and unresolved interaction priorities

React + TypeScript + Vite and Svelte + TypeScript were suggested for comparison.
React was an assistant starting candidate for a dense explorer, not a user
selection. TanStack Table plus a separate virtualizer is one table option:
[Table does not include virtualization itself][ui-table]. Keep domain work in
Rust regardless of which frontend wins. No framework/version/package-manager,
component suite, graph renderer or docking library is selected.

The unanswered product question is which KCachegrind interactions the user
actually relies on: function ranking, callers/callees, source, assembly, call
graph, treemap, and what feels cumbersome or missing. Linked tables/source
panes, a graph canvas, and a docking workspace impose different requirements.
Prioritize concrete workflows before choosing the first UI slice and evaluating
frameworks. Earlier assistant preference for tables/source first is provisional.

#### Binary/debug/source tooling: separate jobs and candidate tradeoffs

Keep these responsibilities distinct:

1. Find the correct executable/shared object and corresponding debug information.
2. Map an address to symbol, file/line and inline frames.
3. Decode instruction bytes for disassembly.
4. Resolve the recorded source path and read actual source text.

The user raised gimli and blazesym. They are not equivalent abstraction levels:
[addr2line][ui-addr2line] builds on gimli and offers DWARF address-to-location/
function/inline-frame queries; [blazesym][ui-blazesym] has a broader symbolization
interface including process/container resolution. Compare higher-level offline
lookup options before deciding direct DWARF manipulation is needed. `object`
is a candidate for object-file/section access; no resolver stack is selected.
Investigate separate/split debug-file lookup and correct asset matching as part
of that comparison rather than assuming all debug packaging works automatically.

Disassembly is independent of symbolization. [iced-x86][ui-iced] is a candidate
for x86/x64; [Capstone's Rust bindings][ui-capstone] are another candidate when
broader architecture coverage matters. Calling installed LLVM/binutils tools
is easy to prototype but adds executable/version/output handling; embedding
libraries gives more controlled behavior but can add native build/packaging
dependencies. A self-contained release was an assistant preference; whether
external tools are acceptable and which architectures matter remain unanswered.
Do not introduce new system dependencies without reconciling existing build
invariants and the reproducible Nix/native-tool setup.

Preserve the findings in [SOURCE-AUDIT.md](docs/SOURCE-AUDIT.md): Valgrind's
instruction positions use the object's linked address space, not automatically
runtime virtual addresses or file offsets. Qualify addresses by object and
verify interpretation before resolving/decoding. Other producers may require
different interpretation; do not blindly apply a Valgrind assumption to all
Callgrind inputs. Debug info enriches metadata; it cannot recover measured
instruction costs from line-only profiling data. Keep observed profile data
distinct from supplementary metadata, and keep file access out of the parser.

#### Testing candidates and intended layers

| Layer | Intended coverage / candidates |
| --- | --- |
| Shared Rust analysis | Exact costs, selection, cycles, source/instruction attribution; reuse existing fixtures and reference infrastructure |
| UI state | Navigation, filtering, selection and history independently of rendering |
| Rendering | Layout, truncation, missing-data states; Ratatui TestBackend + insta is the documented TUI starting point |
| Full applications | Actual startup, input, resizing, loading, shutdown and terminal restoration; browser tests against the real backend |

The user suggested terminal emulation, possibly libghostty-vt. The research
found Ratatui's [snapshot recipe][ui-ratatui] documents TestBackend + insta and
also links [termlens][ui-termlens] for real-binary PTY tests against an emulated
screen. Termlens currently uses vt100 and supplies process management, input,
bounded waits and screen assertions. It is a candidate to investigate before
building a custom harness, not a dependency decision or validated compatibility
claim for this repo.

[libghostty-vt][ui-ghostty] provides terminal parsing/state through C/Zig APIs;
it is an emulator component, not a complete testing harness. Evaluate it against
the actual terminal protocols/behavior needed and the integration/build cost.
A few full-process flows should complement fast state/rendering tests, rather
than making all tests depend on an emulator. TestBackend alone does not exercise
the real terminal lifecycle or event loop. Keep waits bounded and condition-based.

For the browser, [Vitest Browser Mode][ui-vitest] and [Playwright][ui-playwright]
were candidate component and end-to-end tools. Backend query behavior should
also be testable independently of the browser. Framework choice and exact test
dependencies remain open; existing deterministic, network-independent test
requirements apply. No candidate framework has been installed or run here.

#### Next discussion and validation status

Resolve preferred KCachegrind views/first workflow, binary architectures/formats
and remote host support, and acceptability of external tools. Establish the
performance corpus/baseline, then choose concrete libraries against those needs.
Keep remote UI file opening on the deferred list. The prior inclusive-cost
semantics remain in force; this discussion does not reopen or implement them.

Inspected the current remote default branch (`master`, base `0b51303`), working
notes, TODO, handoff, manifests/stubs, source audit and inclusive-cost design.
Read primary library documentation linked below during the discussion; these
are research references, not pinned implementation dependencies. This update
records the discussion only; no new Rust/native/UI tests or benchmarks are
claimed. Documentation diff and local links are checked before publication.

[ui-table]: https://tanstack.com/table/v8/docs/guide/virtualization
[ui-addr2line]: https://github.com/gimli-rs/addr2line
[ui-blazesym]: https://docs.rs/blazesym/latest/blazesym/symbolize/index.html
[ui-iced]: https://github.com/icedland/iced
[ui-capstone]: https://github.com/capstone-rust/capstone-rs
[ui-ratatui]: https://ratatui.rs/recipes/testing/snapshots/
[ui-termlens]: https://github.com/vyncint/termlens
[ui-ghostty]: https://github.com/ghostty-org/ghostling
[ui-vitest]: https://vitest.dev/guide/browser/
[ui-playwright]: https://playwright.dev/docs/test-webserver

### 2026-09-16: distinguish the Rust annotator from upstream

- Renamed the crate directory, package and binary to `callgrind-annotate-rs`.
  The deliberate `-rs` suffix distinguishes this independent implementation
  from upstream's Perl `callgrind_annotate` and avoids confusing bug attribution.
  Updated current commands and directory links; historical prose names remain.
- Publication validation on 2026-09-22: formatting, Clippy, all 169 nextest
  tests, Cargo tests/doctests and 36 Python comparator tests passed. Nix is
  unavailable locally; native integration is checked by the push-triggered CI.

### 2026-09-16: inclusive-cost and cycle design for the shared analysis layer

- User requested a deeper design pass before implementation, with findings
  committed and pushed. Added docs/INCLUSIVE-COST-DESIGN.md: source trace,
  formulas, worked examples, UI/API contract and acceptance criteria.
- Recommended default: exact SCCs on explicitly selected recorded call edges;
  component inclusive = summed self + recorded outgoing boundary costs.
  Internal edges stay queryable but are not repeatedly added. Expanded members
  show self and contribution; individual inclusive is unavailable for members
  of a multi-function SCC. Contributions exactly partition the group formula.
- Structural cycles can arise from independent nonrecursive contexts or a union
  of parts/threads. They do not prove recursion occurred on a measured stack.
  Never propagate a callee's global inclusive cost back to every caller.
- Re-read pinned KCachegrind TracePartFunction/TraceFunction/TraceCall/cycle
  code and globalconfig.cpp. Correction: optional cycle cutoff defaults to 0.0;
  default show-cycles is true. Exact compatibility additionally needs per-part
  caller-count branching and attention to stored versus selected graph edges.
  Existing raw native comparisons disable cycles, so they do not validate a
  cycle-display compatibility mode. This pass did not execute that native UI.
- Preserve annotator policies; no production code changed. Checked the four
  numerical/topological design examples locally. Missing event coverage stays
  unavailable; costs/counts use checked u128; no inferred stack allocation,
  fabricated calls, count-based edge dropping or silently capped percentages.
- Prior docs-only push fd6c191 passed all CI (run 35125949458). Implementation
  evidence remains in SMOKE-TEST.md. Next task remains implementing the shared
  analysis crate against this design, then textgrind and webgrind.

### 2026-09-16: exact flat callgrind2pprof

- User requested the reverse converter and expects every intended change committed
  and pushed. The Work VM had reverted to a pre-publication snapshot; GitHub still
  had verified forward converter tip `4725612805455c5733d72fcd29b461269e415b48`.
  Restored a separate inspection checkout and verified all 109 remote blob hashes
  and modes. Preserved the stale directory rather than overwrite its working files.
- Implemented library plus CLI using the production parser and shared pprof I/O.
  One selected part; self rows only; duplicate locations coalesce deterministically.
  Every stored event survives unscaled. Individual/aggregate/profile totals and
  source lines must fit i64; global bound prevents downstream pprof sum overflow.
  Declared totals mismatch is an error. Summary is descriptive, never cost input.
- Function identity includes object, defining file/name and attributed source file.
  Qualified names prevent pprof display aggregation collisions. Source attribution
  can split one defining function across pprof function entries; never fabricate an
  inline chain. Native locations have one line and zero address/no mapping.
  All original positions, including unsigned 64-bit PCs, survive as string labels.
  Original strings use v: prefix labels to preserve empty values unambiguously.
  Known standard counters use count; sysTime/cache-use/custom units default to
  callgrind_raw with warnings. Explicit --unit overrides declare units, never scale.
- CLI accepts stdin and gzip stdout, --part, --unit, input/location limits; refuses
  overwrite and validates before creating output. Shared gzip writer now flushes
  its underlying writer after finalization so buffered output errors propagate.
- Fourteen converter tests replace the one-field transport smoke test; an extra
  shared flush regression brings the workspace to 169 Rust tests. Python remains
  36. New Go oracle has negative mutation checks. Local independent Go pprof readback
  and actual top reports passed on 41 files / 43 parts, including the recovered
  12-profile SQLite corpus; compares exact event sums and function/source rows.
- Expanded pprof CI waits for the existing SQLite/Nix job, downloads its artifact,
  and converts exactly those files plus 7 focused fixtures and 22 forward outputs.
  Pinned download-artifact v8.0.1; pick latest available artifact attempt so a rerun
  of only the pprof job works. Retain parser TSV/gzip/reports/diagnostics. Existing
  22 KCachegrind comparisons and 93 SQLite annotate comparisons stay intact.
  Expanded native CI is now verified green (completed run below). Qt/Nix
  continue running only on Actions.
- First expanded run `35088328654` exposed a CLI test harness race: invalid
  --unit is rejected before stdin consumption, so the parent's write can return
  BrokenPipe depending on scheduling. The helper now accepts only BrokenPipe
  paired with child failure and still checks stdout/status; a 1 MiB invalid
  invocation exercises early exit reliably. No production semantics changed.
- Follow-up `35088731702` passed Rust/Python and complete SQLite/Nix. The dependent
  job exposed download-artifact v8's single-match flattening (despite pattern and
  merge-multiple=false): files land directly in sqlite-input. Accept that layout
  as well as per-artifact directories when several attempts are downloaded.
  Verified both selector branches locally; no converter changes were required.
- Completed verification: commit `31c8f8fbe04d8b4a14c9445f206a5abd1112e8a5`,
  [run 35089449560](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35089449560), all three jobs/every step green.
  169 Rust / 36 Python tests; 41 files / 43 parts independently read and reported
  by upstream pprof, including the 12 SQLite files generated by this same run.
  All 22 native KCachegrind comparisons and 93 annotation comparisons in both
  app and sandboxed smoke passed; complete flake check succeeded. Seven-day
  artifacts: pprof `10443179842`, SQLite `10443776462`. See SMOKE-TEST.md.
- Next: shared UI analysis, then textgrind/webgrind. Exact caller-stack recovery
  is impossible in general; a future allocation mode must name its approximation.

### 2026-09-16: exact pprof2callgrind implementation

- After the exporter audit, user prioritized implementing our own forward
  converter before the exclusive-cost reverse converter. Added shared complete
  pprof schema/Prost bindings and bounded raw/gzip I/O, reusable streaming
  Callgrind writer, graph/default and context-tree conversion and CLI.
- All value columns retain exact original integers/units. Leaf costs are
  exclusive; graph edges receive each sample once per distinct ordered frame
  pair, including recursive self edges. Tree contexts retain root-prefix paths
  and distinct recursive occurrences. calls=0 means unknown, not fake counts.
- Identity includes mapping/function IDs, PC and line; unknown frames retain
  location IDs. Function suffixes prevent reader name collisions. Context IDs
  are deterministic. Explicit callee objects and absolute positions avoid the
  audited upstream target bugs. Names use injective percent encoding; metadata
  losses, labels, source-path consequences and bounded-memory limits are explicit.
- Negative costs/lines, empty nonzero stacks, width/reference errors and u64
  overflow fail; named output refuses overwrite and opens only after validation.
  `Report` exposes read-only accessors so callers cannot corrupt edge indices.
- Added 19 Rust tests: all-field I/O, gzip CRC/truncation/trailing data/limits,
  exact large/multiple values, graph ambiguity vs trees, recursion, inline and
  unknown identity, writer escaping/state and CLI validation/overwrite behavior.
  All 155 Rust tests and 36 Python tests passed, plus fmt and strict Clippy.
  Upstream Go successfully cross-read all 12 Rust gzip profiles; 22 converted
  outputs passed the input expectations locally (not an independent Qt pass).
- Added separate native Actions gate: hash-pinned Go/pprof, upstream tests and
  probes, Rust cross-read, pinned unmodified KCachegrind libcore. Raw reader
  comparison covers self/edges/source lines, with independent known-stack
  expectations for leaf costs, exact edges and tree paths. Artifacts retained.
  Unlike SQLite/Nix, host Qt/C++ are Ubuntu packages.
- Actual completed verification: implementation `ed99dedbed618ff4265858d8bc1e9494e5e9dbbb`,
  [run 35049704415](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35049704415),
  all three jobs/every step green. Native logs confirm 22 KCachegrind comparisons,
  four upstream Go test packages, pprof probes and cross-read, plus 93 SQLite
  comparisons in both app/sandbox smoke and successful flake check. Artifacts
  `10428103735` (pprof) and `10428790541` (SQLite), retained seven days. See
  `SMOKE-TEST.md`. Full prospective remote tree (109 files, blobs and modes)
  was verified before the non-forced update; implementation worktree was clean.
- Papercut: local apt installation failed on setgroups/setuid permission checks.
  Stopped, asked user, and received approval to run Qt validation on Actions.
  No permission workaround attempted. User explicitly requires no intended work
  left uncommitted or unpushed. Generated profiles/build output stay ignored.
- At this forward-converter checkpoint, reverse `callgrind2pprof` was still a stub
  (now implemented and verified above). Reuse new shared pprof I/O when
  implementing it; do not imply exact full-stack reconstruction from aggregates.

### 2026-09-15: pprof Callgrind exporter audit and reverse-conversion boundary

- User requested this source deep dive before implementing the converter.
  Added `docs/PPROF-CALLGRIND-AUDIT.md`, immutable source links, archive/file
  hashes in `docs/pprof-audit-sources.json`, and optional Go fixture/CLI probes
  under `tests/reference/pprof-audit/`. Linked the existing audit and handoff;
  did not implement `callgrind2pprof` or change production Rust semantics.
- Pinned google/pprof `6331bc6350fe55a6fec2957299e0581dd7510e36`, built with
  SHA-256-verified Go 1.27.1. Traced command overrides, sample selection, graph
  and context-tree construction, recursion, flat/edge formatting, units and
  the real protobuf schema/reader. Four upstream test packages passed without
  source edits: report, graph, driver and profile.
- Our 12 profiles and 32 CLI executions passed their focused assertions.
  Two different full-stack populations produce exactly the same ordinary
  Callgrind bytes, despite different pprof traces. This is a constructive
  non-invertibility proof independent of recursion or numerical precision.
  Pprof computes cumulative values from supplied stacks; flat-only samples
  preserve self values but deliberately lose callers. Approximate allocation
  remains a separately designed feature, not an implied exact inverse.
- Exported edges all use `calls=0`; ordinary graph construction counts each
  node/edge once per sample and suppresses identical-node recursive self edges.
  Production Rust reads the basic output's self total as 100. Unmodified
  Valgrind 3.26.0 Perl annotator calculates 190 by misreading the edge as self.
  Keep our parser's zero-count behavior and do not modify counts to placate Perl.
- Reproduced formatter limitations: wrong relative callee-address basis;
  omitted `cob` causing cross-object edge misqualification; `call_tree` suffixes
  applied to callee names but not function definitions; automatic units losing
  integer precision; 2^53+1 rounding even with count units; means dividing node
  self but not edge weight; explicit-unit divide_by ignored; negative output
  correctly rejected by our unsigned parser. Upstream goldens pass despite
  several of these issues. No upstream issue or patch was filed in this scope.
- Multi-event, label, mapping and period metadata are lost by this one-metric
  writer. Prefer independent protobuf readback plus per-event conservation for
  converter validation. Restrict writer round trips explicitly, and retain
  the ambiguity pair as a regression against overclaiming stack recovery.
- Local formatting, Clippy, nextest (136), Cargo tests/doctests and Python (31)
  passed. Optional audit probes are not automatically run by existing CI and
  Go is not a new Rust development dependency. Existing SQLite CI remains
  unchanged; its next push run checks that this research addition is harmless.
- Papercuts: `go build` only fetched needed modules; `go mod verify` required
  the remaining declared modules too, so the reproduction downloads the pinned
  module graph first, then tests/builds with GOPROXY=off. Web raw-source fetch
  was unavailable; the ordinary pinned upstream archive supplied verified local
  source. Kept downloads, caches and generated reports under ignored `.dev/`.

### 2026-09-15: SQLite CI and Nix verified end to end

- Commit `7a5daa76431c69d8c01974a3689a1a8a5f40a91b`,
  [push run 34999337875](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/34999337875):
  both jobs completed successfully, every step green. Fast CI passed bootstrap,
  formatting, Clippy, 136 nextest tests, Cargo tests/doctests and 31 Python tests.
- Nix generated all 12 profiles, built and tested the workspace (136 tests plus
  doctests), correctly installed both libexec examples, and passed the 31 Python
  tests and production-parser self totals. The writable app and sandboxed smoke
  each passed 93 same-file comparisons (39,597 nonzero function rows). Complete
  flake check passed, reusing those already-built check derivations.
- Artifact `10408578854`, `callgrind-sqlite-1`, uploaded successfully (7,147,308
  bytes, seven-day retention). It contains raw profiles, both CLI reports and
  commands, producer diagnostics and build/comparison/smoke/flake logs.
- This establishes the current Nix toolchain, Rust example install hook, Python
  fixture/import paths and declared runtime closure by actual execution. Nix
  printed a non-failing app `meta` warning. KCachegrind/Qt and source-output byte
  parity remain outside this CI gate. Clang remains a documented future example.
- Updated HANDOFF, TODO, AGENTS and SMOKE-TEST to make the exclusive-cost pprof
  converter the next implementation task. The user keeps direct pushes and
  accepts the Python harness until a later Rust migration.

### 2026-09-15: publication tree restored before verification

- The comparator publication `745bb628` accidentally omitted the unchanged
  files: the tree API received no effective base tree. This also omitted the
  workflow, so no CI run started. The complete local checkout was unaffected.
- Restored the full prior integration tree plus the eight intended changed
  files in follow-up commit `7a5daa76`, preserving both commits and all remote
  ancestry. Compared all 85 tracked paths, blob IDs and modes with the intended
  local snapshot before the non-forced ref update. CI then started normally.
- Publication must assert a nonempty actual base tree and verify the complete
  prospective tree, not only the changed files. Recorded this in AGENTS.md.

### 2026-09-15: first integration run found cross-object display collisions

- Commit `bd31bdf7`, Actions run `34965651354`: fast job green; Nix installed,
  built the pinned SQLite corpus and Rust workspace, passed 136 Nix Rust tests
  and doctests, installed both example binaries correctly, and validated all
  12 profile self totals. The annotation step then failed on duplicate display
  names. Artifact `10394724800` retained all profiles and both failing reports.
- Inspected those bytes and the SHA-256-verified upstream Valgrind 3.26.0 Perl
  source. Loader/libc both define names such as `???:strcmp`; Rust correctly
  retains each object while Perl keys its totals by file:function. Self and
  inclusive rows add exactly to the legacy values. Debug symbols were absent
  from this Nix corpus; the explicit model difference also applies generally.
- Kept production Rust behavior and both raw inputs unchanged. The comparator
  now projects colliding object rows into the documented legacy granularity,
  with exact sums and re-ranking. It validates original row order first and
  still rejects duplicate full display identities. No silent skipping of rows.
  Tree comparison now retains caller, callee and direction before summing
  projected edge counts/costs; this also detects wrong-parent attachment that
  the former context-free edge multiset could miss. A threshold splitting a
  collision group remains a strict mismatch, not an automatically waived case.
- Added a minimal two-object collision profile and negative sum/order/count/
  endpoint tests. All 93 comparisons on the recovered 12-case CI corpus plus
  three fixtures pass locally (39,597 nonzero rows); 31 Python tests pass.
  The full CI/Nix rerun passed; see the newer entry above. Raw object identity
  is still covered independently by Rust tests and the native libcore harness,
  not claimed by this projection.
- Papercuts: active job-log downloads return BlobNotFound until job completion.
  Artifact download succeeded through the GitHub connection; Python urllib's
  default client received HTTP 403/1010 at the returned file URL, while ordinary
  curl downloaded that same authorized URL successfully. No credentials or
  access settings changed. Useful failure artifacts made local reproduction
  possible without rebuilding Nix or re-running the workload.

### 2026-09-15: extensible SQLite integration CI implementation

- User authorized the SQLite/legacy-annotator CI slice and requested an easy
  path to workloads such as profiling Clang. Retained the existing Python
  comparison harness; an eventual Rust migration remains deferred.
- Added `workload/default.nix` registry and a generic JSON plan runner. Plans
  declare executable/argument arrays, stdin, named variants, selected Callgrind
  configurations and process timeouts. Every case gets a private working
  directory; no shell parsing of workload argv. No Clang workload is claimed.
- Preserved all six SQLite configurations at both 64/4096-page cache sizes:
  12 profiles and the existing 87 same-file annotator comparisons. Moved flags
  to one shared configuration table. The SQLite Bash entry point delegates
  to the generic runner. New manifests use `variant` instead of SQLite-specific
  cache-page columns; old manifests remain readable. Output must now be empty.
- Plan/manifest validation rejects wrong case sets, duplicate/missing rows and
  disagreement with the independently supplied Nix plan. Added seven focused
  tests for a non-SQLite command, argument/stdin fidelity, private output files,
  failed workloads, malformed plans, and preserved annotator failure reports.
- Nix separates raw profiles from Rust checks. CI first retains the corpus,
  then the integration app writes both annotators' stdout/stderr/commands to
  the artifact directory. It builds the same sandboxed smoke check and runs
  flake check; both reuse the already-generated corpus. Artifacts/logs upload
  on success or failure for seven days. Initial profile-build failure retains
  build logs; partial failed Nix store outputs are not promised as artifacts.
- Pinned install-nix-action v31.11.1 and upload-artifact v7.0.1 to verified full
  commits. No Rust or nixpkgs/dependency pin changes. The current Work VM lacks
  Nix/native profilers; actual Nix validation will use the configured CI runner.
- Local bootstrap, formatting, Clippy, nextest and Cargo tests passed (136 Rust
  tests); all 28 Python tests and Bash syntax checks pass. Native/Nix/Actions
  results are pending, not inferred from those fast checks.

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
- Published workflow and docs as `e295555053617ad715aee2879475a378154131ed`
  using parent `dd1945e5` and a non-forced update to `master`. The push triggered
  [Actions run 34962686723](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/34962686723),
  which completed **successfully**. All steps in job `104359769092` are green,
  including clean bootstrap, formatting, Clippy, nextest, Cargo and Python tests.
  This establishes the fast CI gate and automatic direct-push trigger. Native
  integration remains deferred; no Nix or KCachegrind CI pass is claimed.
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
  intentional deviations are in `crates/callgrind-annotate-rs/README.md`.
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
