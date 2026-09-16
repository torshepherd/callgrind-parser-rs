# Verified smoke-test results

## Current full gate: both converters, 2026-09-16

Commit `31c8f8fbe04d8b4a14c9445f206a5abd1112e8a5` passed
[run 35089449560](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35089449560).
All three jobs and every step completed successfully:

- Rust/Python: fresh bootstrap, fmt, strict Clippy, 169 nextest tests,
  Cargo tests/doctests and 36 Python tests.
- Reverse conversion: upstream Go pprof independently decoded and checked
  **41 files / 43 parts**, with exact per-event totals, function/source rows,
  original-position labels and flat-sample metadata. Actual upstream top reports
  succeeded for every part. The corpus comprises seven focused fixtures (nine
  parts), 22 forward-converter outputs, and all 12 SQLite profiles downloaded
  from this same run. Oracle mutation/rejection tests passed too.
- Existing forward/native checks: four upstream Go packages, 12 generated
  profiles / 32 CLI probes, 12 Rust gzip cross-reads, and **22 independent
  KCachegrind comparisons passed**.
- SQLite/Nix: all 12 profiles/annotations validated; **93 comparisons passed**
  (39,597 nonzero rows) in both app and sandboxed smoke. Complete flake check
  succeeded using the built derivations.
- Seven-day artifacts: pprof/KCachegrind `10443179842`, SQLite `10443776462`.
  Go/pprof and KCachegrind source pins are unchanged; host Qt/C++ come from
  Ubuntu 24.04. SQLite/Nix pins are unchanged below.

This is exact flat exclusive-cost export, not full-stack reconstruction.
Integer decoded data is the oracle; formatted pprof percentages are not.
Local Rust/Python and independent Go checks also passed. Qt/Nix validation
was performed on Actions.

## Previous forward-converter gate, 2026-09-16

Implementation commit `ed99dedbed618ff4265858d8bc1e9494e5e9dbbb` passed
[Actions run 35049704415](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/35049704415).
All three jobs and every step completed successfully:

- Rust/Python: fresh bootstrap, fmt, strict Clippy, 155 nextest tests,
  Cargo tests/doctests and 36 Python tests.
- Pprof/KCachegrind: four upstream Go packages; 12 generated profiles and
  32 upstream CLI probes; independent Go readback of all 12 Rust gzip outputs;
  negative rejection in both modes; **22 native reader comparisons passed**.
  Exact self/edge/source-line costs and full identities matched unmodified
  KCachegrind libcore, including zero-count recursion, cross-object callees,
  values above 2^53 and multiple columns. Input-derived checks additionally
  verified graph weights and full tree paths. GUI inclusive heuristics excluded.
- SQLite/Nix: all 12 profiles/annotations validated; **93 comparisons passed**
  (39,597 nonzero rows) in both app and sandboxed smoke; flake check succeeded
  using the built derivations. SQLite/Valgrind pins are unchanged below.
- Uploaded seven-day artifacts: pprof/KCachegrind `10428103735`, SQLite
  `10428790541`. The native pprof script is `scripts/check-pprof.sh`; source and
  Go archives are pinned, while Qt/C++ come from the Ubuntu 24.04 runner.

Local Rust/Python and upstream pprof readback also passed. Local Qt installation
was permission-blocked; the user approved native validation on Actions instead.
The results above are actual completed CI execution, not inferred YAML coverage.

## Previous integration gate, 2026-09-15

Commit `7a5daa76431c69d8c01974a3689a1a8a5f40a91b` passed
[Actions run 34999337875](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/34999337875) on Ubuntu 24.04 x86_64.
Both jobs and every step completed successfully. Nix's software pins remain
SQLite 3.51.2, Valgrind 3.26.0 and nixpkgs
`a5cc6f2c37bf518436dc8d1c288ccd0c43c2f4c4`.

| Gate | Observed result |
| --- | --- |
| Fast CI | Fresh pinned bootstrap, formatting, Clippy, 136 nextest tests, Cargo tests/doctests, 31 Python tests passed |
| Nix Rust package | Workspace build, 136 tests, doctests and installed example binaries passed |
| SQLite profiles | Two cache variants times six configurations; 12 profiles, 12 legacy annotations and exact manifest validated |
| Production parser | All 12 profile self totals validated with installed `inspect` |
| Annotator differential | 93 comparisons and 39,597 nonzero function rows passed in both the integration app and sandboxed smoke |
| Full flake | All outputs evaluated and checks passed, reusing the built Rust and smoke derivations |
| Artifacts | `callgrind-sqlite-1`, ID `10408578854`, uploaded successfully; seven-day retention |

The run executed these entry points:

```console
./scripts/nix.sh build .#profiles-sqlite -L
./scripts/nix.sh run -L .#integration-sqlite -- "$PWD/results/checked"
./scripts/nix.sh build .#smoke-sqlite --no-link -L
./scripts/nix.sh flake check -L
```

Both annotators consume the exact same raw bytes. Comparisons check event
order, totals, nonzero function order/costs and call-tree endpoints/counts/costs,
using the documented source grouping and legacy object projection. The 93
runs include 72 SQLite option combinations and 21 focused-fixture comparisons.
Whitespace/percentage display and tree tie order are normalized. Full source
output, object attribution parity in Perl's merged view and KCachegrind/Qt are
outside this gate. Counts are observations, not cross-host goldens.

The first integration attempt exposed loader/libc display-name collisions in
the comparator; the fixed comparator passed the recovered corpus locally and
then this freshly generated CI corpus. See `NOTES.md` for the diagnosis and
[workload/README.md](workload/README.md) for commands and adding workloads.

## Historical producer-only gate, 2026-09-04

The complete flake was built and checked on 2026-09-04 in a fresh x86_64
ChatGPT Linux VM:

```console
./scripts/nix.sh build .#smoke -L
./scripts/nix.sh flake check -L
```

Both commands passed. Nix built the deterministic database fixture, executed
all 12 SQLite/Callgrind combinations, parsed every raw profile with Valgrind's
reference `callgrind_annotate`, and checked the expected file counts and profile
headers.

Locked tools:

- SQLite 3.51.2
- Valgrind/Callgrind 3.26.0
- nixpkgs revision `a5cc6f2c37bf518436dc8d1c288ccd0c43c2f4c4`

Observed sanity signals:

- Switching SQLite from 64 to 4,096 cached pages reduced the `ir-only` total
  from 505,897,944 to 465,311,627 instructions, so the workload genuinely
  exercises SQLite page-cache behavior.
- With the 64-page SQLite cache, simulated D1 read misses were 4,588,427 for
  the constrained cache, 1,578,084 for the balanced cache, and 858,266 for the
  roomy cache.
- In full simulation, the 64-page SQLite cache performed 116,396 system calls;
  the 4,096-page cache performed 1,599.
- The full profile contained instruction/data-cache, branch-prediction,
  cache-use, jump, and syscall event columns, and its reference annotation
  resolved SQLite functions including `sqlite3VdbeExec`,
  `sqlite3BtreeTableMoveto`, `pcache1Fetch`, and `unixRead`.

These totals are evidence from this execution, not cross-machine golden values.
The flake pins the software closure and profiling configuration, but x86_64 CPU
feature dispatch may still change instruction totals between VM hosts. Future
differential tests should pass the same generated profile to both the reference
and Rust annotators.
