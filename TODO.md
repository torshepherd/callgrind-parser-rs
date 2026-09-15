# Next work

Start with [docs/HANDOFF.md](docs/HANDOFF.md). The parser, durable reference
harness and plain-text annotator are committed; implementation baseline is
`d0791129da7c606789193310c66ff4d1978b09f1`.

## Next, in order

- [ ] **Verify fast CI.** `.github/workflows/ci.yml` runs the existing Rust and
  Python checks on default-branch pushes, optional PRs and manual runs.
  Record a completed successful Actions run for the published commit before
  marking this gate complete. Direct pushes remain the normal workflow.
- [ ] **Add native CI and verify Nix.** Deferred to the next slice by the user.
  Add a separate Nix-capable x86_64 integration job, run the existing smoke and
  flake checks, fix actual failures, retain useful CI artifacts, and record a
  completed successful run. The Nix closure remains unverified.
  The handoff lists exact commands, install/import assumptions and acceptance
  criteria. KCachegrind/Qt is a separate gate, not currently part of Nix smoke.
- [ ] **Implement the first callgrind2pprof converter.** Export exact exclusive
  costs from a selected part using the real pprof schema, checked signed-range
  conversion and deterministic gzip/Prost output. Document event units and
  location policy; do not invent full stacks or binary metadata. Test same-input
  conservation and validate with an independent pprof reader. Existing
  `SmokeMessage` tests cover transport only. See the handoff's implementation
  contract before choosing schema dependencies.
- [ ] **Build shared analysis for textgrind/webgrind.** Explicit import/part/thread
  selection and provenance, event-name remapping, checked aggregates, source/
  instruction indexes, reverse calls and exact SCCs. Preserve zero-count edges
  and raw costs. Keep presentation heuristics optional; benchmark before
  replacing storage. Then implement the terminal/browser interfaces.

## Completed milestones

- [x] Callgrind/KCachegrind source audit and parser regressions:
  [docs/SOURCE-AUDIT.md](docs/SOURCE-AUDIT.md).
- [x] Durable SQLite/reference harness, committed as `21df9e9`: all 12 profiles
  and annotations checked, 186,876 raw rows matched, eight focused fixture
  expectations verified. Commands: [tests/reference/README.md](tests/reference/README.md).
- [x] Plain-text annotator, committed as `d079112`: costs/ranking, options,
  call trees and source annotation; 87 same-file semantic comparisons passed.
  Boundaries: [annotator README](crates/callgrind-annotate/README.md).
- [x] Latest observed complete Rust gates: 136 tests, formatting and Clippy;
  Python suite: 21 tests. Native source-pinned results are not a Nix pass.

## Deferred extensions

- [ ] Port the Python reference comparison harness and its tests to Rust.
  User prefers an all-Rust project eventually; Python is accepted for now.
  Preserve same-file comparisons and all existing failure/normalization checks.
- [ ] Annotator derived events, full source-output differential goldens,
  optional cycle-aware views and multipart aggregation, when needed.
- [ ] Decide extended parser dialect scope: nested mangled compression,
  unmangled contexts, basic-block detail, ranges, deprecated `rcalls`, signed
  formulas and old Xdebug costs. Keep explicit rejections until each feature
  has a model and regression tests.

Record decisions, findings and papercuts in `NOTES.md`. Update this checklist
and the handoff when a gate's actual status changes; historical experiments
must not appear as current missing work.
