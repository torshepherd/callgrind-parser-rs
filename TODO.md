# Next work

Start with [docs/HANDOFF.md](docs/HANDOFF.md). The parser, durable reference
harness and plain-text annotator are committed; implementation baseline is
`d0791129da7c606789193310c66ff4d1978b09f1`.

## Next, in order

- [ ] **Add CI and verify Nix.** Separate fast Rust/Python checks from a
  Nix-capable x86_64 integration job. Run the existing smoke and flake checks,
  fix actual failures, retain useful CI artifacts, and record a completed
  successful run for the relevant commit. No workflow files exist at the
  baseline. The Nix closure is unverified; creating YAML is not verification.
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

- [ ] Annotator derived events, full source-output differential goldens,
  optional cycle-aware views and multipart aggregation, when needed.
- [ ] Decide extended parser dialect scope: nested mangled compression,
  unmangled contexts, basic-block detail, ranges, deprecated `rcalls`, signed
  formulas and old Xdebug costs. Keep explicit rejections until each feature
  has a model and regression tests.

Record decisions, findings and papercuts in `NOTES.md`. Update this checklist
and the handoff when a gate's actual status changes; historical experiments
must not appear as current missing work.
