# Same-file reference checks

These adapters compare production Rust parsing with unmodified KCachegrind
libcore at `764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14`. They are test tools,
not a second parser or public analysis API. See `docs/SOURCE-AUDIT.md` for
the semantic contract and `docs/SQLITE-COMPARISON-CHECKPOINT.md` for native
tool versions, archive checksums and build flags.

## Build and deterministic tests

```bash
bash .codex/setup.sh
./scripts/cargo.sh build -p callgrind-parser --examples --locked --offline
python3 -m unittest discover -s tests/reference -v
git clone https://github.com/KDE/kcachegrind.git .dev/reference/kcachegrind
git -C .dev/reference/kcachegrind switch --detach 764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14
bash tests/reference/build-kcachegrind.sh \
  .dev/reference/kcachegrind .dev/reference/kcachegrind-export
```

The build requires a C++17 compiler and Qt6Core development files (`pkg-config`).
It verifies the Git pin and rejects modified/untracked libcore files. An optional
third argument accepts an extracted Ubuntu Qt prefix, containing `usr/include/`
and `usr/lib/x86_64-linux-gnu/`. With that option, export `LD_LIBRARY_PATH` to
that prefix's library directory too: executable RUNPATH alone does not resolve
Qt's transitive dependencies such as libb2. No GUI/moc or system installation
is needed. The adapter passes an **unopened** QFile to `TraceData::load`; that
method opens it. It does not use prefix/sibling-file discovery.

```bash
python3 tests/reference/check_fixtures.py \
  --rust-export target/debug/examples/reference_export \
  --kcachegrind-export .dev/reference/kcachegrind-export
```

Five fixtures compare raw totals, functions, edges and source lines, including
inline files, event remapping, cycles, direct recursion and zero-count calls.
Three assert exact known differences: combined alias lookup diagnostics,
basename identity merging (with conserved attribution), and range syntax.
KCachegrind inclusive rows are diagnostic; recursive inclusive policy is tested
separately. Cycle presentation, jumps and call-target positions are not compared.

## SQLite matrix

Put the pinned Valgrind 3.26.0 tools and actual SQLite 3.51.2 executable (not an
exec-ing wrapper) on PATH, then run from the repository root:

```bash
export LC_ALL=C TZ=UTC
mkdir -p .dev/reference
matrix_root=$(mktemp -d "$PWD/.dev/reference/sqlite-matrix.XXXXXX")
matrix_dir="$matrix_root/profiles"
sqlite3 "$matrix_root/fixture.db" < workload/fixture.sql
test "$(sqlite3 "$matrix_root/fixture.db" 'PRAGMA integrity_check;')" = ok
bash scripts/run-matrix.sh --fixture "$matrix_root/fixture.db" \
  --workload workload/workload.sql.in --out "$matrix_dir"
python3 tests/reference/check_matrix.py "$matrix_dir" \
  --inspect target/debug/examples/inspect \
  --rust-export target/debug/examples/reference_export \
  --kcachegrind-export .dev/reference/kcachegrind-export
```

The native command above now delegates to the generic plan-based runner.
See [the workload guide](../../workload/README.md) for the Nix registry,
non-SQLite commands, CI artifacts and manifest format.

The validator requires the exact six cases at both page-cache sizes, all 12 raw
profiles, 12 annotations with PROGRAM TOTALS, and 12 unique manifest entries
with matching byte counts. Every profile goes through production `inspect`.
Omit both exporter options for parser-only validation; supplying only one fails.
Exporter nonzero status, loader diagnostics, malformed TSV and differences fail
the full comparison. Ordinary loader messages/warnings remain visible on stderr.

TSV tags are P (layout), T (self totals), F (function self), I (reference
inclusive), E (function-pair edge count and inclusive costs), and L (source
line self). Strings are hex UTF-8, counters decimal integers. Event vectors are
aligned by name; dataset-wide zero columns may supplement part-local layouts.
Duplicate rows are rejected before zero-only normalization. A zero-count edge
with positive cost is never dropped. Counter changes and malformed data have
negative tests. Missing names/`???` and line zero are normalized only here.

## Historical reconstruction gate, 2026-09-15

The unchanged native matrix passed all parser totals checks and compared
186,876 rows: T=12, F=9,996, E=20,382, L=156,486. All eight focused fixtures
passed. The 17 Python tests and 98 Rust tests passed; formatting and Clippy
passed. Counts are observations, not cross-host goldens.

At that reconstruction checkpoint, Nix wiring had only been inspected. See
the current CI result in [SMOKE-TEST.md](../../SMOKE-TEST.md); historical native
reference success is a separate gate from a hermetic Nix pass.

## Rust annotator differential checks

```bash
./scripts/cargo.sh build -p callgrind-annotate --locked --offline
python3 tests/reference/check_annotate.py \
  --rust target/debug/callgrind-annotate \
  --reference /path/to/valgrind/bin/callgrind_annotate \
  --matrix "$matrix_dir" --artifacts "$matrix_root/reports"
```

At the initial annotator checkpoint, **87 comparisons passed, 35,080 nonzero
function rows**, including all 12 SQLite profiles, default/self/inclusive
modes, show/sort/threshold
options and direct call trees. That checkpoint had 21 Python tests and 136
Rust tests. The collision regression below extends the comparison coverage.

This compares semantic text tables, not byte-identical output. Rust uses the
explicit `--grouping=source` view to match inline attribution. Object decoration
is projected out because Perl both omits some decorations and merges the
same file:function across different objects. Exact costs are summed and ranks
recomputed only for collisions; original row ordering and duplicate full
display identities are still checked. Tree projection retains both endpoints,
direction and exact call-count/cost sums. Whitespace, percentages, dot-as-zero
and all-zero function rows are normalized. Tree tie order is ignored. Runs use the profile's
directory as cwd to avoid Perl stripping a source prefix from only some tags.
The raw libcore suite independently checks full object/defining-file identity.
Source output and deliberate deviations are covered by focused Rust tests;
full source-output byte parity is not claimed. The annotator guide documents
the precise policy boundaries and machine-report schema.


## Nix collision regression, 2026-09-15

The first integration CI run built the Rust package, passed its 136 tests and
validated all 12 profiles, then exposed loader/libc symbols with identical
`???:name` displays. Perl merged them; Rust correctly kept the objects separate.
The comparator now projects those rows explicitly into Perl's granularity,
without changing either CLI or raw profile. `object-collision.callgrind` and
negative tests cover exact sums, row order, call counts and edge attachment.
On the recovered CI corpus, all **93** annotation comparisons pass locally
(39,597 nonzero function rows). Python tests: **31**.

The full rerun subsequently **passed** at commit `7a5daa76`:
[Actions run 34999337875](https://github.com/torshepherd/callgrind-parser-rs/actions/runs/34999337875).
Both the writable app and sandboxed smoke passed those 93 comparisons on the
newly generated Nix corpus, again observing 39,597 nonzero rows. Production
`inspect` validated all 12 self totals, the Rust package passed its 136 tests,
and complete flake check passed. Both CI jobs and artifact upload were green.
See [SMOKE-TEST.md](../../SMOKE-TEST.md); this does not run KCachegrind/Qt.
