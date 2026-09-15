# Next work

Start with [docs/HANDOFF.md](docs/HANDOFF.md). The parser, durable reference
harness and plain-text annotator are committed; implementation baseline is
`d0791129da7c606789193310c66ff4d1978b09f1`.

## Next, in order

- [ ] **Implement the first callgrind2pprof converter.** Export exact exclusive
  costs from a selected part using the real pprof schema, checked signed-range
  conversion and deterministic gzip/Prost output. Document event units and
  location policy; do not invent full stacks or binary metadata. Test same-input
  conservation and validate with an independent pprof reader. Existing
  `SmokeMessage` tests cover transport only. See the handoff's implementation
  contract and [pprof source audit](docs/PPROF-CALLGRIND-AUDIT.md) before choosing
  schema dependencies or treating its Callgrind exporter as a test oracle.
- [ ] **Build shared analysis for textgrind/webgrind.** Explicit import/part/thread
  selection and provenance, event-name remapping, checked aggregates, source/
  instruction indexes, reverse calls and exact SCCs. Preserve zero-count edges
  and raw costs. Keep presentation heuristics optional; benchmark before
  replacing storage. Then implement the terminal/browser interfaces.

## Completed milestones

- [x] Pprof Callgrind exporter source audit: pinned implementation, four upstream
  test packages, 12 synthetic profiles and 32 real CLI probes. Reproduced
  non-invertibility and specific formatter/reader limitations; no converter
  implementation or new default CI gate is claimed. See
  [docs/PPROF-CALLGRIND-AUDIT.md](docs/PPROF-CALLGRIND-AUDIT.md).
- [x] Extensible SQLite CI and full Nix validation, commit `7a5daa76`:
  [both jobs green](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/34999337875).
  All 12 profiles validated; 93 same-file annotation comparisons passed in
  both the app and sandboxed smoke; complete flake check passed. Profiles,
  both reports and logs uploaded. New workloads reuse the registry and runner;
  see `workload/README.md`. KCachegrind/Qt remains a separate native gate.

- [x] Fast Rust/Python CI, committed as `e295555`: all checks completed green in
  [push run #1](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/34962686723).
  Direct pushes to `master` trigger it automatically; PRs remain optional.
- [x] Callgrind/KCachegrind source audit and parser regressions:
  [docs/SOURCE-AUDIT.md](docs/SOURCE-AUDIT.md).
- [x] Durable SQLite/reference harness, committed as `21df9e9`: all 12 profiles
  and annotations checked, 186,876 raw rows matched, eight focused fixture
  expectations verified. Commands: [tests/reference/README.md](tests/reference/README.md).
- [x] Plain-text annotator, committed as `d079112`: costs/ranking, options,
  call trees and source annotation; 87 same-file semantic comparisons passed.
  Boundaries: [annotator README](crates/callgrind-annotate/README.md).
- [x] Latest observed complete Rust gates: 136 tests, formatting and Clippy;
  Python suite: 31 tests. The current native/Nix result is recorded above.

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
