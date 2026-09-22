# Fresh-session handoff: both converters implemented

Updated 2026-09-21 (frontend planning; implementation/validation unchanged).
Repository: `torshepherd/callgrind-parser-rs`.
Default branch verified for this update: **master**. Resolve it again when resuming.

Rename publication, 2026-09-22: the annotator crate, directory and executable
are now `callgrind-annotate-rs`; commands and the Nix integration runner use that
name. Local formatting, Clippy, 169 nextest tests, Cargo tests/doctests and 36
Python tests passed. All three jobs passed in the rename's
[push run 35771501753](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35771501753)
for `07c0c89ed3ef84a5db0adfcc2d011fbd3b148a0e`, including native integration.

Publication docs now prominently mark the project and all six implemented
crates as experimental, disclose fully LLM-generated documentation, and commit
to a documentation review/cleanup before 1.0. Preserve these notices in the
registry READMEs and crate-level API docs when setting up releases.

First-release setup is prepared at 0.1.0 for six crates, with release-plz,
Trusted Publishing configuration instructions and four-platform cargo-dist /
cargo-binstall workflows. See [RELEASING.md](RELEASING.md). Local gates and
package dry runs passed; actual initial publication, account configuration
and binary/native CI verification remain pending at this checkpoint.

## Start here

The parser, durable reference harness and plain-text annotator are implemented.
**Do not reconstruct the harness or treat the annotator as a stub.** SQLite
integration CI is implemented and verified green. Following the pprof audit,
the user prioritized **pprof2callgrind**, now implemented with exact graph/tree
modes and shared pprof I/O. Read [its guide](../crates/pprof2callgrind/README.md).
The exact exclusive-cost `callgrind2pprof` converter is now implemented too.
Next is shared analysis for `textgrind` and `webgrind`.

Before selecting UI or binary-tooling dependencies, read the
[2026-09-21 discussion](../NOTES.md#2026-09-21-frontend-remote-workflow-binary-tooling-and-testing-discussion).
The user wants requirements/tradeoffs discussed before fixing the stack. The
core use case is a remote profiling box: webgrind through SSH port forwarding
to a local browser, or textgrind directly over SSH. Initial loading uses CLI
paths; opening files in a running UI is deferred. Performance target is as good
as or better than KCachegrind, to be measured on identical profiles including
combined server/browser memory and SSH latency. Axum is the user's proposed
backend; frontend, binary tooling and test frameworks remain unselected.
Preferred KCachegrind views, platform/architecture scope and packaging tradeoffs
are open discussion items. Both UIs remain stubs; no new performance/UI test
results are implied by these notes.

Read [AGENTS.md](../AGENTS.md) first, then this document. Implementation details
and commands live in [the workload guide](../workload/README.md),
[the reference guide](../tests/reference/README.md),
[the annotator guide](../crates/callgrind-annotate-rs/README.md), and
[SOURCE-AUDIT.md](SOURCE-AUDIT.md). Before the converter, also read
[the pprof exporter audit](PPROF-CALLGRIND-AUDIT.md). [TODO.md](../TODO.md) is the ordered checklist;
[NOTES.md](../NOTES.md) retains decisions and papercuts.

## Current CI and development loop

The user works by direct pushes to the verified default branch; **PRs are
optional**. Run local gates, push, inspect that commit's CI, then fix failures
or add a revert commit. Never rewrite shared history. Push triggers cover
`master` and `main` if renamed later; optional PR and manual triggers remain.
CI reports failures after the push and does not automatically roll back.

Three separate Ubuntu 24.04 jobs run:

- Fast Rust/Python: pinned bootstrap, formatting, Clippy, 169 nextest tests,
  Cargo tests/doctests, and 36 Python tests. No custom cache is required.
- Pprof/KCachegrind: pinned Go/pprof and unmodified native reader; 12 input
  profiles, Rust gzip cross-read, 22 graph/tree comparisons, input-derived
  stack/cost expectations. Native CI passed. Qt installation was
  permission-blocked locally; user approved running this check on Actions.
  This job now follows SQLite to reuse its raw profiles and independently
  validate 41 files / 43 reverse-converted parts with upstream pprof.
- Nix SQLite integration: pinned SQLite 3.51.2 and Valgrind 3.26.0, two page-cache
  variants times six configurations, production-parser total validation, both
  annotators on the identical profile, sandboxed smoke, and complete flake check.
  The app and smoke check reuse one generated corpus. Reports and logs upload
  on success or failure for seven days.

The registry is `workload/default.nix`; adding a workload means declaring its
reproducible inputs, executable argv, variants and configurations, then adding
its name to the CI matrix. Shared code provides `profiles-NAME`,
`integration-NAME`, `smoke-NAME` and a flake check. See the workload guide for
commands and a future Clang example; Clang is not an implemented workload.
Python remains accepted for now; an eventual Rust harness migration is deferred.

## Observed validation

- Current completed verification: both converters at
  `31c8f8fbe04d8b4a14c9445f206a5abd1112e8a5`,
  [run 35089449560](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35089449560), **all three jobs/every step green**.
  169 Rust / 36 Python tests; 41 files / 43 reverse-converted parts independently
  validated and reported by upstream pprof, including all 12 freshly generated
  SQLite profiles from this run. All 22 KCachegrind comparisons, 93 annotation
  comparisons in both app and sandboxed smoke, and complete flake check passed.
  Artifacts `10443179842` (pprof) and `10443776462` (SQLite), seven-day retention.
- Previous forward-converter verification: `ed99dedbed618ff4265858d8bc1e9494e5e9dbbb`,
  [run 35049704415](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35049704415),
  **all three jobs and every step green**. 155 Rust / 36 Python tests, four
  upstream Go packages, 12 Rust gzip cross-reads, 22 independent KCachegrind
  comparisons, 12 SQLite profiles, 93 annotator comparisons in app and smoke,
  successful flake check. Artifacts `10428103735` and `10428790541`, seven days.
  See [SMOKE-TEST.md](../SMOKE-TEST.md) for precise scope and pins.
- Local formatting, Clippy, nextest and Cargo tests passed: 169 Rust tests.
  Python suite: 36 tests. On the recovered first-CI corpus, 93 same-file
  annotation comparisons passed, covering 39,597 nonzero function rows.
- Previous full verification: commit `7a5daa76431c69d8c01974a3689a1a8a5f40a91b`,
  [Actions run 34999337875](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/34999337875),
  **passed both jobs and every step**. Native logs confirm 136 Rust tests,
  31 Python tests, all 12 profile totals, and 93 annotation comparisons
  (39,597 nonzero rows) in both the writable app and sandboxed smoke. Complete
  flake check passed using those built derivations. Artifact `10408578854`
  retains the profiles, both reports and logs for seven days. See
  [SMOKE-TEST.md](../SMOKE-TEST.md) for the full validation record.
- First native CI run `34965651354` built the Rust package and validated all
  12 raw profiles, then exposed a comparator restriction on cross-object names.
  The fix preserves production identities and explicitly projects exact costs
  into Perl's file:function view. Tree comparison retains both endpoints and
  exact calls/costs. A collision fixture and negative tests cover this behavior.
- Historical independent KCachegrind libcore gate: 186,876 raw aggregate rows
  and eight focused fixtures passed. That broader SQLite/libcore gate remains
  separate; the new pprof job adds converter-specific Qt/C++ coverage in CI.
  Commands and pins remain in the reference guide and source audit.

The parser supports its documented dialect; a passing differential does not
establish full format conformance, object attribution parity in Perl's merged
view, or source-output byte parity. Instruction totals and row counts are
observations, never cross-host goldens. Compare consumers of identical bytes.

## Maintaining the integration gate

Nix is optional for daily Rust work and unavailable in this Work VM; actual
Nix validation uses the CI runner. Useful local entry points on a Nix host:

```bash
./scripts/nix.sh run -L .#integration-sqlite -- "$PWD/results/sqlite"
./scripts/nix.sh build .#smoke -L
./scripts/nix.sh flake check -L
```

Use an empty results directory. Native dependencies and runtime inputs must
be declared in Nix. Keep the actual profiled binary rather than a launcher
wrapper, the exact variant/configuration cross-product, and same-file reports.
The Rust package installs `inspect` and `reference_export` under `libexec`.
The integration app supplies parser fixture paths explicitly for store imports.
The observed run verified both the example install hook and the sandboxed
runtime closure; those are no longer outstanding Nix assumptions.

Inspect push runs through the repository workflow-runs endpoint with `head_sha`
and `event=push`; the connector's commit-workflow-runs helper filters to PR
runs. Completed job logs and retained artifacts let failures be reproduced
locally even without Nix. A source-only change or a dispatched run is not a
verified gate; record the completed run and its scope.

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
  an unrelated snapshot root or force-update the branch. Require a nonempty
  actual base tree, then compare every prospective tracked path, blob ID and
  file mode against the intended complete checkout before updating the ref.
- Commit useful working slices promptly; generated outputs belong in ignored
  scratch/build directories. Record the actual commit and gate results.

The last interruption also made the execution server unavailable. GitHub API
access still worked and completed the annotator push. A missing execution
server is not evidence that remote work was lost. Do not claim a new test pass
unless a new test actually ran.

## Completed slice: pprof2callgrind

`pprof-profile` contains the full checked-in schema/Prost bindings and validated,
bounded raw/gzip I/O. `callgrind-writer` streams explicit identities, absolute
PCs, escaped names and exact self totals. The converter supports graph/default
and context-tree modes, recursion/inlining, every event column, checked u64
aggregation and negative rejection. Calls=0 means unknown, never invented.
Metadata losses and identity suffixes are documented in its README.

Local fmt/Clippy/nextest/Cargo gates passed (155 Rust tests), Python 36; pinned
Go independently read all 12 Rust-reencoded profiles. Both conversion modes
pass input-derived fixture checks. Native Qt checks run on CI, not this Work VM.
Do not retry apt with sandbox bypasses. The user approved Actions instead and
explicitly requires every intended change committed and pushed before handoff.

## Completed slice: callgrind2pprof

The reverse converter now implements the exact-self contract using the shared
pprof schema and gzip I/O. See [its README](../crates/callgrind2pprof/README.md)
for CLI, limits and detailed policies. Select one part explicitly if multipart.
Only self costs become one-location samples; call edges/counts and jumps are
excluded. All stored events retain exact integers; totals and locations must
fit signed pprof range. Mismatched declared totals fail; summary is not a cost
source. Standard counters use count; unknown/producer-dependent units stay
callgrind_raw unless explicitly overridden without scaling.

Object/defining-function/source identity receives deterministic qualified pprof
names. Original strings and all position columns are sample labels; attributed
source file and line also occupy native pprof fields. No runtime PC, mapping,
build ID, inline chain or timing is invented. Missing and literal unknown names
stay distinct. The flat-only limitation is explicit in both comments and stderr.

Local validation: 169 Rust tests, 36 Python tests, fmt/Clippy/Cargo gates; upstream
Go readback and actual report commands passed for 41 files / 43 parts, including
12 recovered SQLite profiles. The native CI job now follows SQLite to download
and convert the exact newly validated corpus. Expanded native CI passed on
41 files / 43 parts in [run 35089449560](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35089449560).

## Next task: shared UI analysis

Read [INCLUSIVE-COST-DESIGN.md](INCLUSIVE-COST-DESIGN.md) before implementing
inclusive metrics or cycle views. The design pass is complete; implementation
is still pending. Default: exact SCCs over selected recorded edges; component
inclusive = component self + outgoing boundary costs. Expanded cycle members
show self and contribution, with individual inclusive explicitly unavailable.
Keep raw internal edges and annotator compatibility policies separate. The
note specifies selection/event coverage, API/UI behavior and acceptance tests;
no measured stack reconstruction or heuristic cycle cutting is implied.

Build a reusable analysis crate for textgrind/webgrind before either UI:
explicit import/part/thread selection, provenance, event-name remapping, checked
aggregates, source/instruction indexes, reverse calls and exact SCCs. Preserve
zero-count edges and keep raw self/call/jump costs separate. Define the analysis
API with focused tests, then build terminal and browser views on it. Benchmark
before replacing parser storage. UI rendering is not constrained to the
annotator's deliberately barebones text.

Full-stack allocation from Callgrind remains a separately designed approximation,
not an unfinished step in the exact flat converter. The ambiguity proof in
PPROF-CALLGRIND-AUDIT.md still applies. Never describe the converters as lossless
inverses. Both share pprof-profile; do not create duplicate protobuf bindings.

## Later work and established boundaries

Shared UI analysis is next: explicit selection/import
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
UI analysis or trigger a parser rewrite.
