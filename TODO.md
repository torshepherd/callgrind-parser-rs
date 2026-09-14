# Next work

- [ ] **Next: deep source audit of Callgrind and KCachegrind.** Pin the source
  revisions examined and map Callgrind's writer, name/context compression,
  association records, and dump lifecycle to KCachegrind's loader and data
  model. Then study aggregation, inclusive/self costs, recursion and cycle
  handling, source/disassembly attribution, and `callgrind_annotate` behavior.
  Compare findings with our model and tests; record documentation/producer
  disagreements explicitly and add minimal regression fixtures. The targeted
  writer/loader checks during the parser-design review are not this full audit.
- [ ] Run the Rust parser over all 12 profiles from the pinned SQLite matrix;
  check event widths, identities, associations, and self-cost totals.
- [ ] Define annotation compatibility and differential checks on identical
  input files before implementing the annotate frontend.
- [ ] Define the pprof conversion policy for aggregate call graphs: preserve
  exclusive counts without double-counting inclusive edges, document any stack
  approximation, check signed protobuf ranges, and validate with an independent
  pprof reader.
- [ ] Build shared analysis indexes for the terminal and browser frontends;
  cover repeated rows, multiple parts/threads, recursion, and cycles first.
- [ ] Add GitHub Actions using the Cargo bootstrap for fast Rust checks and a
  Nix-capable job for the native reference matrix.

Design reasoning and papercuts belong in `NOTES.md`; this file is the ordered
checklist. Update both when a checked-off task changes a design decision.
