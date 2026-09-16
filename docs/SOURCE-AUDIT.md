# Callgrind / KCachegrind source audit

Audited 2026-09-14 against parser commit `2f6628c261d77555a3f15751919706117fd1a2ca`.

Companion audit (2026-09-15): [pprof's Callgrind exporter](PPROF-CALLGRIND-AUDIT.md)
traces weighted stacks into self/edge rows, reproduces non-invertibility and
formatter fidelity problems, and narrows the oracle for the reverse converter.
The original results below remain their dated checkpoint; current integration
validation is recorded in [SMOKE-TEST.md](../SMOKE-TEST.md).

## Decision

Keep the single incremental decoder and the owned, interned semantic model.
They fit annotation, pprof export, and terminal/browser inspection. Do not
replace them with KCachegrind's object graph. Build shared analysis on top:
explicit part selection, event remapping, checked aggregation, call-site and
source indexes, and a cycle-aware graph view.

The audit found one directly reproduced parser bug: combined dumps lost the
effective process ID and command after their first part. This change fixes it.
It also fixes the reference `compare` example's assumption that independently
generated files have identical record order. Unsupported producer dialects and
analysis policies remain explicit follow-up work, not silently claimed support.

## Revisions and scope

| Component | Pinned source |
| --- | --- |
| Callgrind producer, annotation script, format documentation | [Valgrind 3.26.0 archive](https://sourceware.org/pub/valgrind/valgrind-3.26.0.tar.bz2), SHA-256 `8d54c717029106f1644aadaf802ab9692e53d93dd015cbd19e74190eba616bd7` |
| KCachegrind | [KDE commit 764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14](https://github.com/KDE/kcachegrind/tree/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14) |
| Rust baseline | [2f6628c](https://github.com/torshepherd/callgrind-parser-rs/commit/2f6628c261d77555a3f15751919706117fd1a2ca) |

[source-audit-sources.json](source-audit-sources.json) records SHA-256 hashes of
the examined files. Valgrind references below are paths and function names in
that archive. KCachegrind references use immutable Git links. These are the
revisions examined, not a claim to have reviewed every later upstream change.

Scope: writer and collection/dump lifecycle, compression and associations,
reader state, identities and storage, cost aggregation and cycles, annotation,
and source/disassembly attribution. Instrumentation/JIT correctness, complete
cache simulation, GUI rendering, and a security audit are outside this pass.
KCachegrind findings are from source inspection; Qt/CMake development tooling
was unavailable, so its loader and GUI were not executed here. Callgrind and
`callgrind_annotate` findings below include native executions.

## Source map

| Concern | Callgrind | KCachegrind |
| --- | --- | --- |
| Wire output | `dump.c`: `print_fn_pos`, `print_mangled_fn`, `fprint_pos`, `fprint_fcost`, `fprint_jcc` | [`cachegrindloader.cpp`][loader]: `loadInternal`, context setters, `parsePosition` |
| Dump/interval boundaries | `dump.c`: `new_dumpfile`, `prepare_dump`, `cs_addCount`, `print_bbccs_of_thread`, `dump_profile`; `events.c`: `add_diff_cost`; `main.c`, `callstack.c`, `threads.c` | `prepareNewPart`; [`tracedata.cpp`][data]: `TraceData::load`, `addPart`, `activateParts` |
| Storage and identity | `bbcc.c`, `callstack.c`, `dump.c` context and edge records | `TraceData::function`, `TracePartFunction`; [`fixcost.cpp`][fix], [`fixcost.h`][fixh], [`pool.cpp`][pool] |
| Aggregation and cycles | Inclusive edge costs are already collected | `TracePartFunction::update`, `TraceFunction::update`, `cycleDFS`, `TraceFunctionCycle::setup` |
| Event mapping | `events.c`, `dump.c`: event mapping and emitted columns | [`eventtype.cpp`][events], [`costitem.cpp`][costs] |
| Annotation | `callgrind_annotate.in`: `read_input_file`, `print_summary_and_fn_totals`, source annotation routines | [`sourceview.cpp`][source], [`instrview.cpp`][instr] |

## 1. Producer records and dump lifecycle

Self rows and call rows represent different quantities. `fprint_fcost` emits
exclusive costs, adds them into `dump_total_cost`, and zeroes the emitted cost
buffer. `fprint_jcc` emits an already-inclusive edge cost and clears that edge's
cost and call counter. Never add both kinds when computing program self totals;
never multiply a call-cost vector by the call count. Counts and costs are both
aggregates over the interval.

`new_dumpfile` computes `summary` from changes in collected thread costs;
`close_dumpfile` emits `totals` accumulated from actual self rows. These are
distinct quantities. The manual allows summary to exceed the visible self
costs. Keep both declarations and independently compute self totals. Missing
totals do not mean zero, and a mismatch should be a validation diagnostic,
not a reason to rewrite the profile.

Dumping is incremental. `cs_addCount` accounts for still-active calls before
writing; `add_diff_cost` updates its old-cost baseline. A call that spans two
dumps can consequently have positive cost but **zero new calls** in the later
part. This occurred in the native workload: eight such records across the
three-part combined file. The parser already represents it correctly; a new
minimal fixture protects it. Any future average-cost display must treat
division by zero as unavailable, not discard the edge.

Ordinary dumps are separate files; requested dumps append a part suffix and
thread separation adds a thread suffix. With `--combine-dumps=yes`,
`new_dumpfile` appends parts, retaining name aliases and omitting the repeated
file preamble, `pid`, `cmd`, and cache descriptions. It emits each part's own
positions, events, summary, timerange and trigger. Thread collection may be
merged into thread 1, while active stacks and summary accounting still span
the actual threads. Do not interpret absence of `thread` as evidence that the
program was single-threaded.

**Fixed:** `Decoder::end_part` now carries effective `pid` and `command` forward
until replaced. Thread ID, part number, descriptions, event definitions/layout,
summary/totals, positions and body context still reset. Compression aliases
remain file-scoped. The metadata docs explicitly say these two fields are
effective values; this semantic model does not record whether they were
written or inherited. Cache descriptions remain available on the earlier part;
a future profile-level metadata view should expose stable descriptions without
accidentally inheriting a prior timerange or trigger.

Do not concatenate independently produced files as text. Use a future import
layer that resets wire dictionaries per file, interns into a destination
dataset, and preserves file/run/part provenance. Repeated part numbers and
cross-process identities need explicit handling. KCachegrind's `TraceData::load`
also has prefix-based sibling-file discovery; that belongs in an importer,
not in `parse_reader`.

## 2. Compression, context, and grammar differences

The format's name aliases run from definition to end-of-file. Object names use
one namespace; all file tags share another; function tags share a third.
Sparse maps remain appropriate. Valgrind can give two IDs to the same filename
when it occurs in different objects, so wire IDs must not become semantic IDs.

KCachegrind's `prepareNewPart` clears its compression tables, whereas the
producer retains them when appending combined dumps. This is a source-level
reader/producer disagreement. Follow the producer and manual for alias scope.
KCachegrind also caches *function objects* for function aliases, whereas our
decoder expands names and constructs qualified identities from context. This
distinction matters for unusual inputs that reuse an alias under a different
object/file. It is a compatibility choice, not proof that an alias globally
identifies one Rust `FunctionId`. [Loader source][loader]

`fi`/`fe` change the attribution file within a function; `fl` supplies the
default file restored by the next `fn`. Callee object/file fall back to the
current context and reset after a call record. Jump destination overrides
reset after their association. Our source location/file split is appropriate.

Valgrind `fprint_jcc` writes `jcnd=taken/executed target`, or
`jump=count target`, followed by a source-position row with no event costs.
Its `calls=count target` is followed by source positions and inclusive costs.
Both sides of an association use the previous source-position basis when
relative encoding is used; reading a target must not overwrite that basis.
The manual's formal jump grammar instead uses two space-separated conditional
counts and omits the extra source row. Keep the actual producer dialect and
record this disagreement. The existing grammar suite covers the paired rows.

| Input feature | Finding and current policy |
| --- | --- |
| `instr bb line` columns | The producer supports all ordered combinations. Our parser supports all three. KCachegrind's loader detects only `instr` and `line`, so it is not the oracle for `bb` layouts. |
| `--dump-bb=yes` | Produces the ordinary `bb` position column; native files parsed with matching self totals. |
| `--dump-bbs=yes` | Separate detail dialect emitting `bb=` and `ln=`; currently rejected. Do not confuse it with `--dump-bb`. |
| `--compress-mangled=yes` | Can emit nested function references such as `fn=(2) (1)'2`. Explicitly rejected. KCachegrind's name path stores the remaining name text; it does not implement a nested context decoder. |
| Ordinary context/recursion suffixes | `--separate-callers=2 --separate-recs=3` works with normal compression: suffixes remain part of the opaque function name. They are not yet structured call-stack metadata. |
| `--mangle-names=no` | Legacy `rec=`/`frfn=` context records are rejected. Native output confirmed the gap. |
| `rcalls=` | KCachegrind recognizes this deprecated association with a warning. We reject it pending an explicit compatibility decision. |
| Ranges such as `1:3` | KCachegrind `parsePosition` accepts ranges; our scalar position model rejects them. Supporting ranges needs a representational change and an attribution policy. |
| Signed derived formulas | KCachegrind accepts subtraction/negative coefficients; the published grammar and our `EventTerm` support nonnegative sums. Keep this a documented extension gap. |
| Old negative Xdebug costs | KCachegrind clamps them to zero in `FixCost`. We reject them rather than silently alter counters. |
| Malformed input | KCachegrind often logs and continues with unknown placeholders. Our decoder returns a located error and fuses. That is intentional. |

The range, nested compression, legacy context, and basic-block detail cases
now have small rejection contracts in `tests/source_audit.rs`. They document
the boundary; they do not make the unsupported features complete.

## 3. Identity, memory, and the analysis layer

KCachegrind's `TraceData::function` attempts to qualify functions by name,
file, and object, but its key concatenates the name and the two *basenames*.
That can merge distinct paths or ambiguous concatenations. Its own comment
also explains the format's imperfect definition-file attribution. Keep our
tuple of full interned values; the audit adds a collision regression. Do not
claim that these values establish DWARF-level identity for every producer.
[Function factory][identity]

KCachegrind stores compact `FixCost`, `FixCallCost`, and `FixJump` records in a
pool, then materializes source-line and instruction indexes lazily. Its
`USE_FIXCOST` path is enabled. The important lesson is separation of immutable
observations from indexes and cached totals, not reproducing its pointer-rich
class hierarchy. Our frozen dictionaries, dense event vectors, and streaming
decoder already provide that separation. [Fixed records][fixh], [pool][pool],
[lazy source maps][data]

The streaming decoder retains dictionaries and part metadata, but not prior
cost records. The owned profile intentionally retains rows, including repeats.
That is sensible for all three consumers. The remaining scaling risk is one
boxed cost vector per self/call row, plus record/index overhead. Benchmark
representative large files before choosing flat arenas, sparse tails, or
additional specialized containers. KCachegrind's sparse trailing storage and
200-slot real-event array are implementation choices, not format limits.

Recommended shared analysis contract:

1. Import profiles with stable input/run/part provenance and explicit selection.
2. Map each selected part's event names to a unified event layout before adding
   vectors. Column 0 in one part need not be column 0 in another. Preserve
   whether an event was absent rather than presenting absence as a measurement.
3. Accumulate checked self totals by function and source/instruction location;
   accumulate edges by source location, callee, and recorded target location.
   Build reverse edges and coarser function-pair indexes as views.
4. Keep counts, inclusive edge costs, self costs, summaries, totals, and jumps
   separately queryable. Source and instruction views are two projections of
   the same costs, never two contributions to a function total.
5. Evaluate derived events with explicit unknown-reference, cycle, and overflow
   diagnostics. KCachegrind recursively expands formulas and skips unknown
   names; copying that tolerance would conceal broken definitions.
6. Cache aggregate results by part selection and policy. Repeated source rows
   sum; selecting/deselecting parts invalidates only the dependent views.

## 4. Inclusive costs and cycles are policies

KCachegrind's `TracePartFunction::update` prefers incoming call costs when its
summed incoming **call count is positive**, excluding direct self-recursive
edges. Otherwise it uses self plus outgoing non-self-recursive edges. Its
comment explicitly acknowledges problems involving spontaneous calls, signal
handlers and skipped PLT cost attribution. This is a display convention, not
a universal identity that our parser should enforce. [Implementation][inclusive]

For multiple-function cycles it uses Tarjan-style strongly connected components
and a configurable cost-based edge cut. The cut uses the first real event and
a base from incoming edge costs (or inclusive cost). **Follow-up clarification
(2026-09-16):** the base is the maximum incoming edge cost and the configured
cutoff defaults to 0.0, so cost-based cutting is optional, not always active.
See [globalconfig.cpp](https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/globalconfig.cpp#L20-L27).
The [inclusive-cost design](INCLUSIVE-COST-DESIGN.md) traces cycle/member
accounting and specifies the default shared-analysis policy. It constructs synthetic calls
from a cycle node to its members. Member inclusive displays exclude internal
cycle edges, and cycle self cost is the sum of member self costs.
[Cycle detection and update][cycles]

For our canonical graph, preserve every observed edge and compute exact SCCs
over the selected data. Keep direct recursion visible. An SCC's self total
is the checked sum of member self totals; internal inclusive edges must not
be repeatedly added into it. A KCachegrind-style cycle-cut or inclusive-cost
view can be an explicit presentation policy. Do not recursively sum already
inclusive edge weights as if the graph were a tree, or label a heuristic
allocation as exact measured stacks.

The small `recursive-cost.callgrind` fixture has main self 2, recur self 7,
main-to-recur cost 7, and recur-to-recur cost 5. Actual `callgrind_annotate
--inclusive=yes` prints main 9 and recur 12. The inspected KCachegrind
per-function rule excludes the recursive edge and yields recur 7 before any
multi-function-cycle handling. Tests should name the policy being asserted.

KCachegrind's [`coverage.cpp`][coverage] propagates proportional coverage through
the aggregate graph with recursion guards. These are useful exploratory views;
they do not recover the original call stacks.

## 5. What annotation compatibility should mean

`callgrind_annotate.in` is useful reference behavior, but not a complete format
validator. Its reader ends the header at the first `events:` line, uses
`file:function` keys, changes that key for inline source files, and does not
support multipart files. It ignores target positions and jump information.
The archived manual explicitly states the one-part restriction.

The zero-call-count fixture exposes a stronger limitation. The script decides
whether the next numeric row is a call cost using `curr_call_counter > 0`.
For `calls=0`, it consequently adds the inclusive row to caller self cost.
In the committed fixture the real self costs are main 2, phase 7, total 9;
the native script prints main 9, phase 7, while still printing PROGRAM TOTALS 9.
Our parser preserves the call association and sums self to 9.

In inclusive mode the script first adds outgoing call costs to function costs,
then replaces functions that have incoming call records with their incoming
sum, including recursion. Its summary fallback is nonzero `summary`, then
nonzero `totals`, then summed function results. With inclusive mode and no
declaration that fallback can double-count across functions.

Its ranking is descending by the chosen event columns with a lexical function
name tie-break. Thresholds are cumulative percentages; an explicit threshold
of 100 bypasses truncation. Defaults include threshold 99, source context 8,
percentages on, and auto-annotation on. Source annotation and tree rows need
their own golden cases, including missing sources, inline attribution, and
source-path trimming. Use the script's input and invocation verbatim for those
cases; do not use cross-host absolute event totals as goldens.

Recommended first compatibility target: semantic tables for single-part inputs
using standard event layouts, with optional reference formatting. Document
exceptions for zero-count edges, cycles, basename/object identity and inline
grouping. Preserve correct raw semantics in the shared model; a compatibility
renderer can deliberately emulate a reference quirk if needed. Compare both
tools against the *same* bytes. A green reference exit code alone is insufficient.

## 6. Source, disassembly, and pprof implications

Valgrind `get_debug_pos` emits instruction addresses after subtracting the
object relocation offset (`dump.c`, lines 391-393; `bb.c` obtains the text bias).
They are addresses in an
object's linked address space; do not automatically treat them as runtime
virtual addresses or ELF byte offsets. Keep object qualification alongside
addresses, particularly for merging multiple profiles.

Source text, instruction bytes, load mappings, build IDs, and full inline
chains are not supplied by these rows. The disabled `mp=` block in `dump.c`
is not emitted mapping metadata. KCachegrind finds source files separately
and invokes `objdump` on the corresponding binary for disassembly. Its lazy
line maps omit line zero and may hide jumps to the same or immediately following
line; those are UI choices and should not remove our raw records.
[Source lookup][source], [disassembly lookup][instr]

Both frontends need an optional source/binary resolver with path remapping,
input provenance and explicit availability. The parser should never read files
named in a profile or run a disassembler. Unknown source line zero should remain
representable; a browser upload without source/binary assets is still a useful
profile. An actual inline stack requires supplementary debug information.

For pprof, keep the earlier policy constraint: aggregate call edges do not
determine full sampled stacks, even when bounded caller-context suffixes exist.
An initial exact-self export can emit single-location samples for exclusive
rows, coalesced as appropriate. Document lost call-path detail. Any deeper stack
allocation needs a separately named approximation and conservation tests.
Never add inclusive edges as additional samples, invent runtime mappings, or
cast overflowing unsigned values to signed protobuf fields. Event units need
an explicit map; syscall time even has mode-dependent units (`sysTime` long
names in `new_dumpfile`). Gzip/Prost transport does not establish conversion
correctness; independent pprof validation remains a separate gate.

The [pprof companion audit](PPROF-CALLGRIND-AUDIT.md) now confirms this with
two distinct sample populations that its ordinary exporter maps to identical
Callgrind bytes. It also reproduces `calls=0`/Perl double counting, recursion
deduplication, unit/integer loss, and target/object/context-name problems in
the pinned formatter. Use the upstream profile reader and decoded sums as
independent checks; its Callgrind writer is only a scoped oracle.

## Validation and follow-up

The committed [native workload](../workload/source-audit.c) makes two dump
requests inside an active function, then terminates. Seven native configurations
were run against the recovered Valgrind 3.26.0 build:

| Configuration | Observation |
| --- | --- |
| Combined, ordinary compression | Three parts; self totals 203708, 453, 1649; eight zero-count/nonzero-cost call records |
| Combined, string/position compression off | Same self totals and 5525 semantic entries, including three part-layout entries |
| Caller depth 2 / recursion separation 3 | All three separate files parsed and passed self/totals checks |
| `--dump-bb=yes` | All three separate files parsed and passed self/totals checks |
| Nested mangled compression | All three files rejected as `UnsupportedExtension` |
| `--dump-bbs=yes` | All three files rejected as `UnknownBodyLine` |
| `--mangle-names=no` with caller/recursion separation | All three files rejected as `UnknownBodyLine` |

These totals are observations, not portable goldens. Independent runs emitted
functions in different orders because `dump.c` sorts contexts using producer
pointer addresses. `examples/compare.rs` now compares a sorted multiset of
resolved records within each part, retaining duplicates and part boundaries.
It still excludes process metadata and input line numbers and requires
deterministic counters. The comparison succeeded for all 5525 entries.

Five new active tests cover metadata inheritance and part resets, active
zero-count calls, recursive cost separation, identity collisions, and explicit
unsupported syntax. The metadata test failed with `None` instead of `Some(42)`
before the fix. `callgrind_annotate` was executed on both minimal fixtures.
KCachegrind behavior above remains source-derived, not a runtime differential.
Formatting, Clippy with warnings denied, nextest and Cargo tests passed:
98 tests total, including 95 parser tests, using locked/offline Cargo commands.
Nix and the 12-profile SQLite matrix were not rerun in this audit.

Native reproduction (from the repository root; `VG_SRC` points to an already
configured and built Valgrind 3.26.0 source tree):

```bash
mkdir -p .dev/reference/source-audit
cc -static -g -O0 -fno-inline -I "$VG_SRC/callgrind" -I "$VG_SRC/include" \
  workload/source-audit.c -o .dev/reference/source-audit/workload
"$VG_SRC/vg-in-place" --tool=callgrind --error-exitcode=99 \
  --collect-jumps=yes --dump-instr=yes --cache-sim=no --branch-sim=no \
  --combine-dumps=yes \
  --callgrind-out-file=.dev/reference/source-audit/combined \
  .dev/reference/source-audit/workload
./scripts/cargo.sh run -p callgrind-parser --example inspect --locked --offline -- \
  .dev/reference/source-audit/combined
perl "$VG_SRC/callgrind/callgrind_annotate" --auto=no --threshold=100 \
  --show-percs=no crates/callgrind-parser/tests/fixtures/zero-call-count.callgrind
perl "$VG_SRC/callgrind/callgrind_annotate" --auto=no --inclusive=yes \
  --threshold=100 --show-percs=no \
  crates/callgrind-parser/tests/fixtures/recursive-cost.callgrind
```

For the other configurations, substitute the table's options and distinct
output paths. For the uncompressed counterpart add `--compress-strings=no
--compress-pos=no` and compare with the `compare` example. The static link is
a host-specific native smoke-test convenience, not a new Cargo requirement.

The next gates are the 12-profile SQLite matrix, the shared analysis contract
and its adversarial graph fixtures, and a headless KCachegrind differential
harness. Address source-audited dialect gaps deliberately; do not block the
ordinary producer path on every legacy extension. See [TODO.md](../TODO.md).

[loader]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/cachegrindloader.cpp
[data]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/tracedata.cpp
[identity]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/tracedata.cpp#L3566-L3627
[inclusive]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/tracedata.cpp#L770-L863
[cycles]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/tracedata.cpp#L2321-L2513
[fix]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/fixcost.cpp
[fixh]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/fixcost.h
[pool]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/pool.cpp
[events]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/eventtype.cpp
[costs]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/costitem.cpp
[coverage]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/coverage.cpp
[source]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libviews/sourceview.cpp
[instr]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libviews/instrview.cpp
