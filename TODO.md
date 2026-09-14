# Next work

- [x] **Callgrind/KCachegrind source audit.** See
  [docs/SOURCE-AUDIT.md](docs/SOURCE-AUDIT.md) for pinned sources, producer/reader
  disagreements, model decisions, native experiments and minimal regressions.
  KCachegrind was inspected, not executed; runtime parity remains below.
- [ ] **Finish and commit the SQLite/reference test harness.** All 12 native
  profiles passed parser/self-total checks and 186,924 aggregate rows matched
  KCachegrind, but the environment disappeared before implementation commits
  and final gates. See [the checkpoint](docs/SQLITE-COMPARISON-CHECKPOINT.md).
  Native source-pinned runs are not a rerun of the Nix closure.
- [ ] Define annotation compatibility and differential checks on identical
  single-part input files before implementing the annotate frontend. Start
  with semantic tables; explicitly cover zero-count calls, recursive inclusive
  costs, summary fallback, inline grouping, object identity and ranking ties.
- [ ] Define the pprof conversion policy for aggregate call graphs: preserve
  exclusive counts without double-counting inclusive edges, document any stack
  approximation, check signed protobuf ranges, and validate with an independent
  pprof reader.
- [ ] Build shared analysis indexes for the terminal and browser frontends;
  cover repeated rows, event-name remapping, multiple files/parts/threads,
  checked overflow, zero-count calls, recursion, and exact SCCs first. Preserve
  provenance and raw edges; make KCachegrind's inclusive/cycle-cut conventions
  optional display policies. Add lazy source/instruction indexes and a separate
  source/binary resolver. Benchmark before replacing per-record cost storage.
- [ ] Build a headless harness against the pinned KCachegrind libcore for
  runtime checks of combined-file compression scope, identities, inline
  attribution, ranges, event remapping and cycle costs.
- [ ] Decide dialect scope from the audit: nested mangled compression,
  unmangled context tags, basic-block detail records, ranges, deprecated
  `rcalls`, signed event formulas and old Xdebug costs. Keep explicit rejections
  until each extension has a model and regression tests.
- [ ] Add GitHub Actions using the Cargo bootstrap for fast Rust checks and a
  Nix-capable job for the native reference matrix.

Design reasoning and papercuts belong in `NOTES.md`; this file is the ordered
checklist. Update both when a checked-off task changes a design decision.
