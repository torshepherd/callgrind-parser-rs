# Next work

Start a fresh session with [docs/HANDOFF.md](docs/HANDOFF.md). The harness has
been reconstructed and the plain-text annotator implemented. Native/Rust gates
pass; the Nix closure remains unverified.

- [ ] Run the updated `smoke` build and `flake check` on a Nix-capable host.

- [x] **Callgrind/KCachegrind source audit.** See
  [docs/SOURCE-AUDIT.md](docs/SOURCE-AUDIT.md) for pinned sources, producer/reader
  disagreements, model decisions, native experiments and minimal regressions.
  KCachegrind was source-inspected in that audit; subsequent native runtime
  results are recorded in the checkpoint below.
- [x] **Finish and commit the SQLite/reference test harness.** Reconstructed
  and rerun 2026-09-15: 186,876 rows match; eight fixtures and 17 Python tests
  pass. See `tests/reference/README.md`. Nix wiring is still unverified.
  Historical checkpoint: all 12 native
  profiles passed parser/self-total checks and 186,924 aggregate rows matched
  KCachegrind, but the environment disappeared before implementation commits
  and final gates. See [the handoff](docs/HANDOFF.md) for reconstruction and
  [the checkpoint](docs/SQLITE-COMPARISON-CHECKPOINT.md) for historical evidence.
  Native source-pinned runs are not a rerun of the Nix closure.
- [x] Implement the plain-text annotator and define compatibility/differential checks on identical
  single-part input files before implementing the annotate frontend. Start
  with semantic tables; explicitly cover zero-count calls, recursive inclusive
  costs, summary fallback, inline grouping, object identity and ranking ties.
  See `crates/callgrind-annotate/README.md`; 87 native same-file comparisons pass.
- [ ] Extend annotation coverage if needed: full source-output differential
  goldens, derived events, optional cycle-aware views and multi-part aggregation.
  Do not conflate these extensions with the implemented single-part port.
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
- [x] Build a headless harness against the pinned KCachegrind libcore for
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
