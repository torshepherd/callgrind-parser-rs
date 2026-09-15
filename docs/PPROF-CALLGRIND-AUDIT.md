# pprof's Callgrind exporter: source audit

Audited 2026-09-15. This complements the
[Callgrind/KCachegrind audit](SOURCE-AUDIT.md), especially sections 1, 4 and 6.
It is research for `callgrind2pprof`, not an implementation of that converter.

## Conclusion

Yes: Google pprof has a `-callgrind` output command. It projects weighted
sampled stacks into a graph, then writes node **flat/self costs** and edge
weights. Pprof already computes cumulative costs from stacks; the difficult
part of the reverse conversion is obtaining those stacks, not implementing
inclusive summation ourselves.

The ordinary exporter is not invertible. Two different stack populations in
our executable probe produce **byte-identical Callgrind output**. It also loses
sample labels, all but one selected value column, real call counts, and most
mapping metadata. Its current formatter has additional fidelity problems.
Use it as a complementary producer and scoped test oracle, not as a lossless
serialization of `profile.proto` or an oracle for all Valgrind semantics.

Keep the initial exact-self converter plan. Single-location samples preserve
exclusive values but deliberately lack caller attribution. A later synthetic
stack mode must state its allocation policy and conservation guarantees; it
cannot claim to recover the original measured stacks in general.

## Revision, source map and execution

Pinned upstream: [google/pprof `6331bc6350fe55a6fec2957299e0581dd7510e36`][pprof]
(2026-09-06). This audit is about that Go implementation, not the older
gperftools Perl pprof or every version vendored by `go tool pprof`.
[Source hashes](pprof-audit-sources.json) identify the inspected files and
downloaded archive. Sources were unmodified.

| Concern | Source and entry points |
| --- | --- |
| CLI and overrides | [`commands.go`][commands]: `callgrind`, `kcachegrind`; [`driver.go`][driver]: `generateRawReport`, `applyCommandOverrides`, `aggregate`, `sampleFormat` |
| Report/formatter | [`report.go`][report]: `newGraph`, `newTrimmedGraph`, `selectOutputUnit`, `printCallgrind`, `getDisambiguatedNames`, `callgrindAddress` |
| Stack aggregation | [`graph.go`][graph]: `newGraph`, `newTree`, `CreateNodes`, `nodeInfo`, `AddToEdgeDiv` |
| Value formatting | [`measurement.go`][measurement]: `Scale`, `convertUnit`, `autoScale` |
| Wire schema and reader | [`profile.proto`][schema]; [`profile.go`][profile]: `ParseData`, `CheckValid`, `Write`; [`index.go`][index]: sample-type selection |
| Upstream tests | [`driver_test.go`][drivertest]: `TestParse`; [`report_test.go`][reporttest]: `TestDisambiguation`; CPU, heap and call-tree golden reports |

Built the pinned CLI with SHA-256-verified Go 1.27.1, without changing its
`go.mod`/`go.sum`. The module declares Go 1.25.0. Tests passed in
`internal/report`, `internal/graph`, `internal/driver` and `profile`, including
the existing Callgrind goldens. Our separate [audit probe][probes] constructs
12 small profiles through upstream `profile`, validates and writes/reads them,
then invokes the actual CLI 32 times. It retains inputs, decoded profiles,
commands, stdout and stderr. It is an optional research gate, not a new Go
dependency for Cargo or a new default CI workload.

The production Rust parser and the unmodified Valgrind 3.26.0 annotator were
also run against selected emitted files. No KCachegrind/Qt execution was added
in this audit. A passing upstream golden test does not establish semantic
agreement with independent readers.

## 1. From samples to the output graph

The CLI selects one `Sample.value` column using `-sample_index` or the profile's
default selection (normally its preferred type, otherwise the final column).
`-mean` uses column zero as the divisor. Values already carry their measurement;
the graph does not multiply each value by `Profile.period` to obtain its cost.

`-callgrind` forces address granularity and disables normal node/edge/count
trimming. It does **not** disable prior focus/ignore/hide/show filters, frame
pruning, label-root/leaf insertion, or `-noinlines`. Exporting a filtered report
is not exporting the original unmodified observations. In our `-hide=foo`
probe, the remaining `main` frame receives all 100 units as flat cost.

For the ordinary graph, `newGraph` processes each selected sample as follows:

1. Its protobuf locations are leaf-first; traversal runs from root toward leaf.
   Each location's inline chain is expanded in the same outer-to-inner order.
2. Locations merge by `NodeInfo`, not protobuf ID. This includes address, source
   line/column, function display name, file and (for Callgrind) object filename
   and function start line. Distinct IDs with identical attributes can merge.
3. Add the sample weight to each visited node's cumulative total **once per
   sample**, using `seenNode` to avoid recursion double counting.
4. Add the weight to each distinct ordered edge **once per sample**, using
   `seenEdge`. Identical-node self edges are suppressed. Distinct call-site
   locations can still be different nodes within the same named function.
5. Add flat cost only to the retained leaf. Samples with both selected weight
   and mean divisor zero are skipped; zero-total nodes are subsequently omitted.

Edge weights therefore measure selected sample value flowing through a stack
adjacency, not dynamic invocation counts. A sample representing 90 instructions
contributes 90 to each retained edge it traverses, not 90 extra units of self
cost per edge. `Cum` is computed by graph construction, but `printCallgrind`
does not serialize a node's cumulative number as another self row.

Our `basic` example produces the expected pprof top report:

| Function | Flat | Cumulative |
| --- | ---: | ---: |
| `main` | 10 | 100 |
| `foo` | 90 | 90 |

The `flat` example retains the same values but uses separate one-location
samples: `main`'s cumulative becomes 10, while its flat cost stays 10. This is
the exact limitation of the proposed first converter, not a pprof bug.

### Recursion is also a policy

For leaf-first `[recur, recur, main]` with weight 7 plus `main` self 2, pprof's
ordinary graph gives `recur` flat/cumulative 7, `main` cumulative 9, and a
single `main -> recur` edge of 7. The identical-location recursive self edge is
absent. A longer `A -> B -> A -> B -> A` path contributes once to `A -> B` and
once to `B -> A`, not once per dynamic occurrence.

These are sampled-stack graph conventions. Valgrind's instrumented inclusive
edges can count repeated recursive execution differently. Neither pprof's nor
KCachegrind's cumulative view should become a parser invariant.

## 2. What `printCallgrind` actually writes

| Output | Meaning or omission at the audited revision |
| --- | --- |
| `positions: instr line` | Two position columns, even when a position is unknown/zero |
| `events: TYPE(UNIT)` | Exactly one selected and possibly scaled metric; no original multi-column layout |
| `ob`, `fl`, `fn` | Object/source/display names, compressed with separate dictionaries; empty names are blank values |
| Ordinary cost row | Node `FlatValue()` after unit conversion and conversion to `int64` |
| `cfl`, `cfn`, `calls=0 ...` | Callee source/name and position, with a deliberately unknown call count encoded as zero |
| Following `* * COST` | Caller position plus the edge's selected weight, already aggregated |
| Not written | `cob`, `summary`, `totals`, PID/thread/part metadata, labels, period/duration, mapping ranges/offsets/build IDs, jump records, inline-edge markers |

The address comes directly from `Location.Address`; no relocation or mapping
offset conversion makes it equivalent to Valgrind's object-linked addresses.
Mapping filenames survive as object names, but the information needed to
interpret runtime addresses in their mappings does not. Column and function
start-line information participates in graph identity but is not written as
distinct fields. System/mangled names are not separately preserved.

`calls=0` is particularly important for this project. The Rust parser correctly
keeps its following cost as an edge. In the `basic` output it computes self
total **100**. The reference Perl `callgrind_annotate` uses its positive-count
test instead, treats that edge as self, and calculates **190** (main 100, foo
90). Unlike the earlier fixture, there is no declared summary to mask the bad
fallback total. This independently reproduces the limitation in the original
audit; do not change zeros to ones to appease Perl.

## 3. A constructive proof that the ordinary export cannot be inverted

Each of these profiles has two samples of weight 4. Stacks below are shown
root-first for readability; the probe stores them leaf-first in protobuf.

| Profile P | Profile Q |
| --- | --- |
| `main -> A -> M -> X` | `main -> B -> M -> X` |
| `main -> B -> M -> Y` | `main -> A -> M -> Y` |

Both have X self 4, Y self 4, and the same six edge weights:
`main->A=4`, `main->B=4`, `A->M=4`, `B->M=4`, `M->X=4`, `M->Y=4`.
There is no recursion, overflow or unit conversion needed for this ambiguity.
The CLI's `-traces` outputs differ, but its ordinary `-callgrind -unit=count`
outputs are **byte-for-byte equal**, asserted by the probe.

Thus even exact node costs and exact edge marginals do not retain which
ancestor was associated with which descendant through shared M. Actual call
counts would not distinguish this example either: both executions can have
the same edge counts. Recovering one plausible stack decomposition is different
from identifying the original decomposition. Proportional flow allocation is
an assumption, not an inverse operation.

For multiple event columns, allocating each independently could additionally
invent incompatible cross-event sample correlations. A future stack mode
needs a joint model and explicit ambiguity/cycle handling, not just a recursive
walk that re-adds inclusive edge weights.

## 4. Formatter-specific fidelity problems reproduced

These are separate from the intrinsic information loss above. They describe
the pinned exporter, not new behavior that our parser should emulate.

### Relative target addresses use the wrong earlier node

`callgrindAddress` receives `prevInfo` for both the current self row and its
callees. But after emitting that self row, the current source position is the
Callgrind relative-position basis. In `basic`, foo is at `0x1100` and main at
`0x1000`; the output correctly moves to main with `-256`, then says
`calls=0 * 20`. That target `*` refers to `0x1000`, not foo's `0x1100`.
The writer computed it relative to the previously emitted foo node instead.
The source-location `* *` row is fine; the target encoding is wrong.

### Cross-object edges omit the callee object

The `objects` profile places main in `/audit/app` and foo in `/audit/lib.so`.
The writer emits each node's `ob`, but no `cob` for the cross-object call.
Our production semantic exporter consequently finds foo self 90 in lib.so,
but a main-to-foo edge of 90 whose callee is qualified by **app**. A reused
function-name alias does not substitute for callee object context. Some readers'
function-alias caches can conceal this, as discussed in the earlier audit.

### `-call_tree` has context information internally, but broken name wiring

`newTree` uses parent-specific node maps, preserving separate call contexts
and recursive occurrences. `getDisambiguatedNames` creates names such as
`recur [1/2]` and `recur [2/2]`. This is a useful design reference for a producer
that already has measured stacks.

However, `printCallgrind` applies these names to **cfn only**; it writes raw
`n.Info.Name` for `fn` and groups rows by that raw name. Our recursive probe
defines self rows under `recur`, then emits edges to suffixed names without
corresponding self definitions. The Rust exporter confirms the split identities.
The upstream call-tree golden contains this pattern too. Its disambiguation
helper test and golden pass; neither asserts complete reader-side tree identity.
Do not recommend this switch as a currently verified lossless escape hatch.

### Scaling, large integers, means and division

| Probe | Observed result |
| --- | --- |
| Two self values 1500 ns and 2500 ns, default units | Chooses microseconds and writes 1 + 2 = 3 us, losing 1 us from the original 4 us |
| Same profile with `-unit=ns` | Writes 1500 + 2500 exactly |
| Self value 2^53 + 1, explicit count units | Protobuf readback and CLI raw output preserve 9007199254740993; Callgrind writes 9007199254740992 |
| Values `[samples=2, cpu=100 ns]`, `-mean` | Self becomes 50, but outgoing edge stays 100 under the mean event header |
| Same CPU profile, explicit ns and `-divide_by=2` | Output is unchanged from the undivided Callgrind report |
| Negative self value -5 | Writes a negative cost; our unsigned Callgrind dialect correctly rejects it with `InvalidNumber` |

`measurement.Scale` returns `float64`, even when no meaningful unit change is
needed, and the formatter casts it to `int64`. `FlatValue()` uses the mean
divisor, whereas the edge formatter reads raw `Weight` rather than
`WeightValue()`. The formatter also bypasses `Report.formatValue`, which is
where normal report formatting applies `Ratio` for `divide_by`.

Explicit source units avoid the small-value scaling loss but **do not** make
this writer a full-range integer oracle. Exported negative difference profiles
are not evidence that the Callgrind unsigned-counter model should silently
clamp or reinterpret them.

## 5. Implications for our converter and future tests

1. **Use real `profile.proto`.** It allows mapping-less locations, independent
   location/function IDs, signed sample values and multiple value types. Put
   exclusive measured values into samples once. `Period` is not a substitute
   for sample values. Do not invent mappings, inline chains or runtime PCs.
2. **Validate structurally and semantically.** Upstream `ParseData`/`CheckValid`
   is useful independent validation, not a proof of conservation. Also compare
   decoded per-event sums and selected flat reports to parser self totals.
   Check IDs, references, vector widths and signed ranges ourselves.
3. **Test flat-only limitations explicitly.** Single-location samples make
   flat and cumulative equal at that location. They do not reconstruct the
   original caller graph. Function aggregation may merge display identities,
   so raw decoded identity checks must accompany human-facing top reports.
4. **Use a restricted round-trip oracle.** For `pprof -> Callgrind -> Rust`,
   select one event, set exact source units, disable filtering/means/ratios,
   use nonnegative small integers, and avoid known identity/target pitfalls.
   Check self costs separately from edges and positions. A discrepancy outside
   this subset is not automatically a bug in our converter/parser.
5. **Preserve the synthetic ambiguity pair.** It prevents a later implementation
   from claiming exact stack recovery merely because graph totals match.
   Add adversarial recursion, zero-count, cross-object, inline and multi-event
   cases to any proposed approximate allocation policy.
6. **Do not change production parsing to copy exporter bugs.** Keep the existing
   Rust/Perl SQLite suite's scope; Perl is especially unsuitable as the oracle
   for pprof's zero-call-count exports. No new Callgrind dialect or CLI behavior
   was introduced by this research.

Pprof's current input parser has protobuf and several legacy CPU/heap/thread/
contention/Java readers, but no Callgrind reader. There is no existing inverse
in this pinned package to delegate to. The exporter is valuable prior art,
not a replacement for designing `callgrind2pprof`.

[pprof]: https://github.com/google/pprof/tree/6331bc6350fe55a6fec2957299e0581dd7510e36
[commands]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/internal/driver/commands.go
[driver]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/internal/driver/driver.go
[report]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/internal/report/report.go#L928-L1066
[graph]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/internal/graph/graph.go
[measurement]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/internal/measurement/measurement.go
[schema]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/proto/profile.proto
[profile]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/profile/profile.go
[index]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/profile/index.go
[drivertest]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/internal/driver/driver_test.go
[reporttest]: https://github.com/google/pprof/blob/6331bc6350fe55a6fec2957299e0581dd7510e36/internal/report/report_test.go
[probes]: ../tests/reference/pprof-audit/README.md
