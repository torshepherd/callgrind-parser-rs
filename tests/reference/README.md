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
matrix_dir=$(mktemp -d "$PWD/.dev/reference/sqlite-matrix.XXXXXX")
sqlite3 "$matrix_dir/fixture.db" < workload/fixture.sql
test "$(sqlite3 "$matrix_dir/fixture.db" 'PRAGMA integrity_check;')" = ok
bash scripts/run-matrix.sh --fixture "$matrix_dir/fixture.db" \
  --workload workload/workload.sql.in --out "$matrix_dir"
python3 tests/reference/check_matrix.py "$matrix_dir" \
  --inspect target/debug/examples/inspect \
  --rust-export target/debug/examples/reference_export \
  --kcachegrind-export .dev/reference/kcachegrind-export
```

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

## Observed reconstruction gate, 2026-09-15

The unchanged native matrix passed all parser totals checks and compared
186,876 rows: T=12, F=9,996, E=20,382, L=156,486. All eight focused fixtures
passed. The 17 Python tests and 98 Rust tests passed; formatting and Clippy
passed. Counts are observations, not cross-host goldens.

The Nix smoke derivation now installs Rust examples and invokes the same
matrix validator plus Python tests. Its pinned install hook was inspected,
but **Nix is unavailable here, so the Nix build/check is unverified**. Run
`./scripts/nix.sh build .#smoke -L` and `./scripts/nix.sh flake check -L` on a
Nix-capable host. Native reference success is not a hermetic Nix pass.
