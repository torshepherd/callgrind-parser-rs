# callgrind-annotate-rs

> **Experimental — documentation status:** This crate is experimental. Its
> documentation is fully LLM-generated and may contain errors or outdated claims.
> The documentation will receive a review and cleanup pass before the 1.0 release.

The deliberate `-rs` suffix distinguishes this independent Rust implementation
from upstream's Perl `callgrind_annotate` and avoids confusing bug attribution.
Keep the package and binary names distinct from upstream.

A plain-text Rust alternative to Valgrind's `callgrind_annotate`, using the
shared production parser. No colors, interactive UI, external Perl dependency,
or new Callgrind grammar implementation. It prints aligned cost columns,
function rankings, optional caller/callee rows and annotated source.

```bash
./scripts/cargo.sh run -p callgrind-annotate-rs --locked --offline -- \
  --auto=no --show=Ir,Dr --sort=Ir --threshold=99 profile.callgrind

./scripts/cargo.sh run -p callgrind-annotate-rs --locked --offline -- \
  --inclusive=yes --tree=both --show-percs=no profile.callgrind

# Source lookup defaults to recorded paths; add search roots as needed.
./scripts/cargo.sh run -p callgrind-annotate-rs --locked --offline -- \
  --context=8 -I path/to/sources profile.callgrind source.c
```

## Options

| Option | Behavior / default |
| --- | --- |
| `--show=Ir,Dr` | Selected recorded events in requested order; default all |
| `--sort=Ir,Dr` | Descending multi-column sort; default recorded event order |
| `--threshold=99` | Exclusive cumulative cutoff; 100 shows all functions |
| `--sort=Ir:99,Dr:90` | Per-event cutoffs override the global threshold |
| `--show-percs=yes\|no` | Percentages of program totals; default yes |
| `--inclusive=yes\|no` | Function inclusive-cost policy below; default no |
| `--tree=none\|caller\|calling\|both` | Direct aggregate edges; default none |
| `--auto=yes\|no` | Auto-select attributed sources for ranked functions; default yes |
| `--context=N` | Source context lines, merged without duplication; default 8 |
| `-I DIR`, `--include=DIR` | Repeatable source search roots |
| `--part=INDEX` | Zero-based input index; required for multipart input |
| `--grouping=function\|source` | Defining-file identity (default) or Perl-style attribution grouping |
| `--format=text\|tsv` | Plain text (default) or exact machine-readable report |

Input defaults to `callgrind.out`. Extra positional arguments select source
files even with `--auto=no`. A missing explicit source is an error; unavailable
automatic sources are listed without failing. `-I` also searches the relative
tail of an absolute recorded path below each supplied root. File names must
match recorded attribution names; there is no basename guessing.

## Semantics and compatibility

- Function identity is `(object, defining file, name)`, using full paths.
  `--grouping=source` explicitly splits function costs by attribution file,
  matching Perl's `fi`/`fe` grouping for ordinary profiles. It still retains
  object identity. Neither view alters the underlying parser/model.
- Self rows determine self costs; calls contain already-inclusive costs.
  Calls are never multiplied by call count or added into self totals.
  Zero-count/positive-cost edges remain visible, including `(0x)` tree rows.
- Inclusive function cost is the sum of incoming recorded edge costs if
  incoming edges exist, otherwise self plus outgoing edges. This is the
  annotator's presentation policy, **not** cycle detection or measured stacks.
  Direct recursive incoming edges count, so the recursive fixture shows 12,
  unlike KCachegrind's 7. Multi-function cycles are not recursively expanded.
- Program totals use nonzero summary, then nonzero declared totals, then
  summed **self** costs. The last fallback intentionally avoids Perl's
  inclusive-mode double counting. A declared-total mismatch warns; it does
  not silently rewrite declarations. Zero-count calls intentionally fix the
  upstream script's self-cost misclassification.
- Ranking uses exact u128 counters, then lexical `file:function`, then full
  identity for deterministic ties. Aggregation/count addition is checked.
  Percentages/cutoffs use floating point for display/selection only; a global
  threshold of 100 bypasses cutoff calculations. Inclusive cutoff follows
  the upstream policy: after each row, progress is program total minus that
  row's inclusive cost, rather than a cumulative sum of inclusive rows.
- Source rows sum only attributed self costs. Call-site rows retain each
  caller/callee pair, even when different functions share a source line.
  Line zero and beyond-EOF costs are shown separately. Annotation coverage
  counts actual printed valid self lines, not call costs or unavailable lines.
- Multipart files require explicit selection rather than silently summing
  possibly overlapping intervals/threads. Derived formulas are not evaluated;
  selecting an unrecorded event gives an error. Parser dialect limits remain.

Compatibility is **semantic, not byte-for-byte**. Rendering uses plain aligned
text but prints explicit zeros, stable file ordering, source line numbers,
and full file paths. Unlike Perl it does not selectively strip the current
directory or drop object labels from inline rows. Source mtime warnings,
Perl's mutable annotation-denominator quirks, and whitespace are not emulated.

The same-file reference suite compares source-grouped function costs/ranking,
program totals and direct tree edge costs/counts. It normalizes whitespace,
percent decorations, dot-as-zero and all-zero function rows. Perl merges
`file:function` across objects, so the comparator explicitly projects Rust's
separate object rows into that view, adding exact costs and re-ranking while
checking original ordering. Tree projection keeps both endpoints and exact
call-count/cost sums. Duplicate full display identities still fail. This is
comparison-only normalization; production object identity is unchanged and
is independently checked by the Rust/KCachegrind harness.
It runs away from source directories to avoid Perl's inconsistent cwd trimming.
Focused tests protect intentional deviations, source display and CLI failures.

## Machine report

`--format=tsv` does not open source files. Counters are exact decimal integers;
names are hexadecimal UTF-8, with absent names represented as empty fields.
Vectors follow `--show` order. Tags:

- `P`: report part index (always 0), comma-separated hex event names.
- `T`: index, program totals (summary precedence above).
- `F`: index, object/file/name, selected self or inclusive costs; ranked/filtered.
- `E`: index, caller object/file/name, callee object/file/name, count, edge costs.
- `S`: index, attribution file, line, self costs across functions.

E/S include all observed edges/source rows in the selected input part, not just
ranked functions. This is a report, not lossless profile serialization; use the
parser model for provenance, jumps, target positions, and absent-name fidelity.

## Validation

```bash
./scripts/cargo.sh test -p callgrind-annotate-rs --locked --offline
./scripts/cargo.sh build -p callgrind-annotate-rs --locked --offline
python3 tests/reference/check_annotate.py \
  --rust target/debug/callgrind-annotate-rs \
  --reference /path/to/valgrind/bin/callgrind_annotate \
  --matrix /path/to/the/12-case-matrix
```

See `tests/reference/README.md` for tool pins and matrix generation. Native
comparison and hermetic Nix results must be reported separately.
