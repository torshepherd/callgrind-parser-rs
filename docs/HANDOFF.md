# Fresh-session handoff: pprof2callgrind implemented

Updated 2026-09-16. Repository: `torshepherd/callgrind-parser-rs`.
Default branch verified for this update: **master**. Resolve it again when resuming.

## Start here

The parser, durable reference harness and plain-text annotator are implemented.
**Do not reconstruct the harness or treat the annotator as a stub.** SQLite
integration CI is implemented and verified green. Following the pprof audit,
the user prioritized **pprof2callgrind**, now implemented with exact graph/tree
modes and shared pprof I/O. Read [its guide](../crates/pprof2callgrind/README.md).
Next, implement the first
exclusive-cost `callgrind2pprof` converter, then shared analysis for `textgrind`
and `webgrind`.

Read [AGENTS.md](../AGENTS.md) first, then this document. Implementation details
and commands live in [the workload guide](../workload/README.md),
[the reference guide](../tests/reference/README.md),
[the annotator guide](../crates/callgrind-annotate/README.md), and
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

- Fast Rust/Python: pinned bootstrap, formatting, Clippy, 155 nextest tests,
  Cargo tests/doctests, and 36 Python tests. No custom cache is required.
- Pprof/KCachegrind: pinned Go/pprof and unmodified native reader; 12 input
  profiles, Rust gzip cross-read, 22 graph/tree comparisons, input-derived
  stack/cost expectations. Initial native CI run pending. Qt installation was
  permission-blocked locally; user approved running this check on Actions.
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

- Local formatting, Clippy, nextest and Cargo tests passed: 136 Rust tests.
  Python suite: 31 tests. On the recovered first-CI corpus, 93 same-file
  annotation comparisons passed, covering 39,597 nonzero function rows.
- Completed full verification: commit `7a5daa76431c69d8c01974a3689a1a8a5f40a91b`,
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
  and eight focused fixtures passed. **This Qt/C++ gate is not in current CI.**
  Its commands and pins remain in the reference guide and source audit.

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

## Next task: exclusive-cost reverse pprof converter

The crate already has `callgrind-parser`, `clap`, `prost`, and pure-Rust-backend
`flate2` dependencies. Its one test round-trips a custom one-field
`SmokeMessage` through gzip; **it is not conversion**. Reuse `pprof-profile`
for the real schema/transport rather than maintaining another set of bindings.

Start with [SOURCE-AUDIT.md, section 6](SOURCE-AUDIT.md) and
[PPROF-CALLGRIND-AUDIT.md](PPROF-CALLGRIND-AUDIT.md). The latter pins upstream
`6331bc6350fe55a6fec2957299e0581dd7510e36` and includes executable research
probes: different stacks export to identical ordinary Callgrind bytes. Pprof
computes inclusivity from supplied stacks; single-location samples deliberately
have no callers. Its exporter is not a lossless oracle: zero call counts trip
Perl, scaling/float conversion can lose exact values, and target addresses,
cross-object edges and `call_tree` name wiring have reproduced problems.
These are not reasons to weaken our parser or change counts to one. Use the
upstream reader plus decoded per-event conservation, with a carefully scoped
writer round trip. The Go probes now also participate in the forward converter's
separate native CI gate; they are not reverse conversion. Recommended first scope:

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

Shared UI analysis comes after the converter: explicit selection/import
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
