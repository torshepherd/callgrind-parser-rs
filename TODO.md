# Next work

Start with [docs/HANDOFF.md](docs/HANDOFF.md). The parser, durable reference
harness and plain-text annotator are committed; implementation baseline is
`d0791129da7c606789193310c66ff4d1978b09f1`.

## Next, in order

- [ ] **Finish the frontend requirements discussion before fixing the stack.**
  See [the 2026-09-21 working notes](NOTES.md#2026-09-21-frontend-remote-workflow-binary-tooling-and-testing-discussion).
  Confirm priority KCachegrind views, host/binary architecture and format scope,
  and external-tool packaging tradeoffs. Core workflow: remote webgrind via SSH
  port forwarding or textgrind directly over SSH; startup paths first. Axum is
  the user's proposed backend; frontend, resolver/disassembler and test packages
  remain candidates, not finalized choices.
- [ ] Establish a same-profile KCachegrind performance baseline and corpus:
  loading, peak/steady memory (server plus browser), navigation/filtering and
  responsiveness under SSH latency. Target as good or better than KCachegrind;
  choose numerical budgets after measuring, not an invented file-size limit.
- [ ] **Build shared analysis for textgrind/webgrind.** Explicit import/part/thread
  selection and provenance, event-name remapping, checked aggregates, source/
  instruction indexes, reverse calls and exact SCCs. Preserve zero-count edges
  and raw costs. Keep presentation heuristics optional; benchmark before
  replacing storage. Follow [the inclusive-cost design](docs/INCLUSIVE-COST-DESIGN.md):
  component self plus outgoing boundary costs, explicitly labeled member
  contributions, selected-graph SCCs and documented acceptance cases. Then
  implement the terminal/browser interfaces.

## Completed milestones

- [x] `callgrind2pprof`: exact exclusive-cost flat export, explicit part/units,
  identity and original-position labels, deterministic validated gzip, checked
  signed ranges. Fourteen Rust regressions replace the transport-only stub test;
  shared gzip flushing has another regression. Local upstream readback/report
  validation passed on 41 files / 43 parts, including 12 recovered SQLite files.
  Expanded same-corpus native CI passed on freshly generated SQLite profiles in
  [run 35089449560](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35089449560); all three jobs green.
  See [the guide](crates/callgrind2pprof/README.md).

- [x] `pprof2callgrind`: complete shared pprof bindings/I/O, reusable writer,
  exact multi-event graph and context-tree modes, 19 new Rust tests. Local
  workspace: 155 Rust / 36 Python tests; upstream pprof cross-read passed.
  Independent KCachegrind CI gate: 22 comparisons passed; all three jobs green
  in [run 35049704415](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35049704415).
  Shared `pprof-profile` is now used by both converters.

- [x] Pprof Callgrind exporter source audit: pinned implementation, four upstream
  test packages, 12 synthetic profiles and 32 real CLI probes. Reproduced
  non-invertibility and specific formatter/reader limitations. This research
  preceded the converter milestone above. See
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
  Boundaries: [annotator README](crates/callgrind-annotate-rs/README.md).
- [x] Latest observed complete Rust gates: 169 tests, formatting and Clippy;
  Python suite: 36 tests. The current native/Nix result is recorded above.

## Deferred extensions

- [ ] Open/change profiles and associated assets from an already running web UI.
  Reuse the startup loader; remote filesystem browsing is the natural SSH
  workflow. Uploading browser-local files is a distinct feature, not assumed.
- [ ] Port the Python reference comparison harness and its tests to Rust.
  User prefers an all-Rust project eventually; Python is accepted for now.
  Preserve same-file comparisons and all existing failure/normalization checks.
- [ ] Annotator derived events, full source-output differential goldens,
  optional cycle-aware views and multipart aggregation, when needed.
- [ ] Decide extended parser dialect scope: nested mangled compression,
  unmangled contexts, basic-block detail, ranges, deprecated `rcalls`, signed
  formulas and old Xdebug costs. Keep explicit rejections until each feature
  has a model and regression tests.

## Upstream comparison survey (low priority)

Contrast other Callgrind-format readers; extract anything the audits missed.
Expect little: all three are small, dated, or likely low-effort.

- [ ] https://github.com/compor/CallgrindParser — old PHP project; likely
  little to gain.
- [ ] https://github.com/subroy13/callgrind-reader — ~3 years old; worth
  contrasting only.
- [ ] https://github.com/yaowubarbara/callgrind-parser — recent (~5 months),
  Rust, likely AI-generated; interesting only if it surprises us. Avoid
  getting distracted by it.

## Local machine work (this host, before returning to ChatGPT sessions)

- [ ] Set up local crates.io publishing. `cargo login` is already done on
  this machine; publish the parser crate when ready.
- [ ] Set up a GitHub Actions workflow to publish to crates.io.
- [ ] Set up GitHub Actions with cargo-dist (or equivalent) so the repo is
  `cargo binstall`-friendly.

Record decisions, findings and papercuts in `NOTES.md`. Update this checklist
and the handoff when a gate's actual status changes; historical experiments
must not appear as current missing work.
