# Inclusive costs and cycles: analysis/UI design

Design recommendation, 2026-09-16. For the future shared analysis crate,
textgrind and webgrind. This document specifies behavior to implement; it
does not claim that a new analysis crate or cycle-aware UI already exists.
The parser, annotator and converters keep their current contracts.

## Decision in one paragraph

Preserve measured self rows and inclusive call-edge rows separately. Build
exact strongly connected components (SCCs) over the explicitly selected call
graph, and use **component self plus recorded outgoing boundary costs** for
the default cycle-aware inclusive view. Never recursively add a callee's
whole-program cost. Expand a cycle group into member self costs and **member
contributions**, not supposedly recovered per-member cumulative costs.
Keep incoming, outgoing and internal recorded costs inspectable. Compatibility
with KCachegrind or the annotator is a separately named policy.

## What the sources actually establish

This extends [SOURCE-AUDIT.md](SOURCE-AUDIT.md), using the same pinned
KCachegrind revision, `764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14`.
This pass inspected source; it did not execute KCachegrind's cycle UI.

| Source | Finding |
| --- | --- |
| [TracePartFunction::update][part] | Sums self; prefers incoming costs when total incoming call count is positive, otherwise uses self plus outgoing. Both branches exclude direct self-recursive edges. The comment acknowledges spontaneous-entry and skipped-PLT attribution problems. The branch is chosen per part before aggregation. |
| [TraceFunction::update][function] | Ordinary functions sum active per-part values. Cycle members use self plus outgoing costs excluding edges within their cycle. The cycle's self is the sum of member self. |
| [TraceFunctionCycle::setup][setup] and [TraceCall::update][call] | A cycle creates synthetic links to members; each link obtains that member's cycle-adjusted value. Summing these produces the group's self plus external outgoing costs. These links are presentation objects, not recorded calls. |
| [cycleDFS][dfs] | Tarjan-style grouping with optional small-edge omission. Uses the first real event and a cutoff based on the maximum incoming edge cost, or inclusive cost if there are no callers. It does not create a multi-member cycle for a self loop. |
| [globalconfig.cpp][config] | Cycle display defaults on; the cycle cutoff defaults to **0.0**. The optional heuristic is not an unavoidable part of default grouping. |
| [Valgrind manual, avoiding cycles][manual] | Explains that independent call chains can superimpose into a cycle. Caller-context and recursion separation can avoid some grouping by collecting more identity information. |
| [Our pprof audit](PPROF-CALLGRIND-AUDIT.md) | Edge marginals do not determine complete calling contexts. Pprof's stack-based deduplication is a different policy; do not substitute it for Callgrind edge semantics. |

The previous source audit described the configurable cutoff but did not state
its zero default. That distinction matters. Also, a future KCachegrind
compatibility mode must reproduce per-part incoming-count branching before
aggregation; applying one branch after merging parts is not equivalent.

A selection detail also matters: cycleDFS walks stored call links without an
explicit active-part filter, and a zero cutoff does not discard zero-cost
links. Source inspection therefore does not justify assuming its default SCC
membership equals exact topology restricted to selected parts. Our contract
below explicitly builds topology from the selected records.

A recorded internal edge is still useful measured data. What is unsafe is
interpreting it as a disjoint slice of the group's cost or expanding aggregate
cycles into measured stacks. We will keep these edges available, even where
a reference UI suppresses their inclusive display.

## Three quantities, not one overloaded "inclusive"

For one selected additive stored event, let:

- `S(f)` be the sum of self rows attributed to function f.
- `W(f,g)` be the sum of recorded inclusive call costs from f to g.
- `N(f,g)` be the separately summed recorded call counts.

W already includes work below the call. Do not multiply it by N or walk the
graph to add more descendants into it. N=0 does not remove an edge: a call
spanning a dump can have new cost without a new invocation. Counts are not
a reliable test of whether caller attribution exists.

For an SCC C, including a singleton:

```text
self(C)      = sum S(f), for f in C
outgoing(C)  = sum W(f,g), for f in C and g outside C
incoming(C)  = sum W(f,g), for f outside C and g in C
inclusive(C) = self(C) + outgoing(C)

contribution(f within C) = S(f) + sum W(f,g), for g outside C
sum contributions of C's members = inclusive(C)
```

These are exact checked sums of the selected records. The default inclusive
number is a declared boundary-accounting convention. It is not a new
measurement, proof of complete attribution, unique stack occupancy, or a
guarantee of equality with incoming(C) for every producer.

For a singleton without recursion, this is ordinary self plus outgoing costs.
For a singleton with a self loop, the self-loop cost is internal and excluded
from inclusive(C), but remains in the edge table. A multi-member component
uses the same rule; no special recursive summation is needed.

Do not replace outgoing(C) with the sum of successor components' inclusive
costs. Each successor may have unrelated callers. Do not compute a transitive
sum of reachable self costs either: reachability does not tell us which costs
belong to this caller. The condensation DAG is useful for navigation and
termination, not permission to reattribute its descendants.

A function inside a multi-member component does not get a default individual
inclusive value. The API returns an explicit unavailable reason and its group
ID; the UI offers self and contribution. Contribution is a disjoint breakdown
of the chosen group formula, not that function's total recursive influence.

## Worked cases

### Direct recursion: why 7 and 12 can both appear

Self: main=2, R=7. Edges: main->R=7 and R->R=5.

| View | main | R |
| --- | ---: | ---: |
| Self | 2 | 7 |
| Default cycle-aware inclusive | 9 | 7 |
| Existing annotator inclusive policy | 9 | 12 |

R's self already includes its recorded recursive activations. Adding its
self-recursive edge into the group value counts overlapping work again.
The edge cost 5 stays visible as recorded recursive-call cost. The annotator
result remains intentional compatibility behavior, protected by its existing
test; this design does not silently change that CLI.

### Mutual cycle: a useful exact breakdown

Self: main=1, A=3, B=5, H=2.
Edges: main->A=10, A->B=8, B->A=1, B->H=2.
The component is {A,B}; H is outside it.

| Row | Self | Default inclusive | Contribution to {A,B} |
| --- | ---: | ---: | ---: |
| main | 1 | 11 | — |
| Cycle {A,B} | 8 | 10 | — |
| A, expanded member | 3 | unavailable | 3 |
| B, expanded member | 5 | unavailable | 7 |
| H | 2 | 2 | — |

Group inclusive is 3+5+2=10. Contributions 3 and 7 partition that 10.
A->B=8 and B->A=1 stay in the internal-edge view. B's contribution of 7
deliberately excludes the internal call to A; it must not be labeled "B's
inclusive cost". Group self 8 plus H self 2 plus main self 1 gives program
self total 11. Inclusive rows overlap across groups and never sum to a
program-total denominator.

### Shared callee: even an acyclic graph cannot be expanded blindly

Self: main=0, A=2, B=3, H=5.
Edges: main->A=6, main->B=4, A->H=4, B->H=1.

A inclusive is 2+4=6; B inclusive is 3+1=4; main inclusive is 10.
Giving H's global 5 to both callers incorrectly yields 7+8=15.
Collapsed cycles do not solve this separate loss of calling context.

### A structural cycle need not be a recursive stack

One observation can be main->A->B, another main->B->A, with neither stack
repeating a function. Their union nevertheless has component {A,B}.
The same effect can appear when selecting two phases or threads.

Call it a **cycle in the selected call graph**, not proof of simultaneous
mutual recursion. Preserve per-part witnesses so a later detail view can
explain which observations supplied each direction. Context suffixes already
present in function names remain distinct; never strip them to manufacture
an apparent simpler graph.

## Selection, topology and incomplete data

1. First choose inputs/parts/threads and an explicit identity-merge policy.
   Default to one explicitly selected part. Do not sum overlapping dumps or
   independent runs implicitly; PID and part number alone are not global IDs.
2. Preserve full object/defining-file/name identity and import provenance.
   Inline attribution files belong to locations, not new topology nodes.
3. Build canonical topology from every recorded call edge in the selected
   data, including zero-count and all-zero-cost records. This is structural
   topology, not an inference of nonzero activity for the displayed event.
   Jumps do not become call edges.
4. Find exact SCCs without cost cutoffs. Track self loops on singleton
   components. Aggregate intercomponent edges with references to original
   call sites and entry/exit functions; create no fake calls.
5. Changing the displayed event, sort order, search or visual threshold must
   not change canonical components. Changing the underlying selection or
   explicit identity merge can. Collapse the selected union graph after
   aggregation; summing previously cycle-adjusted per-part values is a
   different operation.
6. A giant component is an honest limitation of the selected aggregate data.
   Allow member search/self ranking and narrower part/thread selection.
   A future edge-cut view must be visibly approximate, preserve the canonical
   graph, and never claim its new boundary costs are measured stack totals.

Map event columns by name and compatible metadata before summation. Missing
events are not measured zero. Initially make a cross-part scalar metric
available only when the event exists in every selected part; otherwise return
explicit incomplete coverage and require narrower selection. An absent
function's rows within a part that does record the event contribute zero.
Ambiguous units/configurations must be surfaced before combining inputs.
Derived ratios, averages and non-additive metrics need separate policies.

Keep declared summary, declared totals and computed self totals distinct.
Compare declarations with self totals as diagnostics; do not repair values.
Show incoming(C) alongside inclusive(C) when investigating attribution gaps.
Differences may reflect partial data or producer policy, and are not by
themselves parser errors. No incoming edges is not evidence of zero work.
Never silently use max(incoming, self+outgoing) or distribute the difference.

## API and UI contract for the first implementation

The proposed module boundary is observations -> selected aggregates -> graph
components -> presentation metrics. This is a responsibility split, not a
requirement for four separate crates.

Use checked u128 accumulators for costs and counts, consistent with the
annotator. Return typed errors for overflow or invalid vector width. Keep
missing metrics distinct from zero. Only presentation percentages use
floating point; comparisons and sorting use integer values.

Suggested result concepts (names are provisional):

| Query/result | Required content |
| --- | --- |
| Selected graph | Selection/provenance, event coverage, full function identity, raw call-site edges and reverse index |
| Component | Members, singleton-self-loop flag, internal/boundary edge references |
| Component costs | Self, incoming, outgoing, boundary inclusive, coverage and policy identifier |
| Member costs | Self, group contribution, individual inclusive or Unavailable(CycleMember), containing group |
| Diagnostics | Declared-total mismatch, incomplete event coverage, incoming/boundary disagreement without inferred repair |

Cache topology by selection and identity policy, and metrics additionally by
event and inclusive policy. Do not expose a bare generic inclusive() whose
meaning changes with the active renderer. Use deterministic component ordering
from member identities rather than DFS encounter order. Avoid recursive DFS
on the Rust call stack for untrusted deep profiles.

Two complementary UI views:

- **Functions:** every original function remains searchable; default ranking
  by self cost. Cycle members link to their group. No sorting unavailable
  inclusive values as zero; inclusive ranking uses the component view.
- **Inclusive call graph:** group multi-member SCCs and show self/inclusive.
  Expanding a group exposes self/contribution plus internal recorded edges.
  Member rows are a breakdown, not extra top-level costs. A singleton with
  a self loop keeps its function name and shows a recursion indicator.

For example, a row can say "Cycle: A, B — self 8, inclusive 10"; expanded
columns say "Self" and "Contribution", with internal edges in a separate
caller/callee list. Source views show self costs with separate call-site rows.
Graph navigation must not imply reconstructed chronological call paths.

Percentages default to the selected data's computed self total for that event,
labeled accordingly. Declared summary is inspectable as a separate quantity.
An optional group-breakdown percentage uses group inclusive as its explicitly
named denominator. Zero denominator means unavailable. Never clamp values
above 100%; explain scope or attribution disagreement instead.

Do not advertise cost/count as an unqualified average per invocation: interval
costs and new-call counts can cover different populations at dump boundaries.
For v1, show them separately; zero-count division is always unavailable.

## Compatibility and alternatives

- **Existing annotator:** retain its documented incoming-edge preference,
  including recursion and zero-count edge fixes. Refactoring shared aggregates
  must not change its ranking, denominator or inclusive behavior accidentally.
- **KCachegrind-compatible display:** optional later. Requires a named version
  policy, per-part caller-count branch, direct recursion rules, cycle settings
  and native tests with cycles enabled. Our current raw KCachegrind comparison
  disables cycles; its green status is not evidence of cycle-display parity.
- **Boundary inclusive (recommended default):** no caller-count branch,
  reproducible sums, useful member partition, handles missing entry edges
  without making self work disappear. Can differ from reference attribution
  around skipped PLT, signals or partial records; differences stay visible.
- **Pprof-style unique stack membership:** cannot generally derive it from
  these aggregate records. Keep it out of the exact analysis API.
- **Proportional allocation/flame graphs:** deferred explicit approximation.
  A visually plausible partition is not recovered calling context.

## Implementation acceptance criteria

Focused tests should protect these behaviors before either UI depends on them:

1. Direct-recursion fixture: self 7, default inclusive 7, raw recursive edge 5,
   annotator policy still 12.
2. Mutual-cycle example: group self 8/inclusive 10, member contributions 3+7,
   internal edges retained; no synthetic recorded calls.
3. Shared-callee example: 6 and 4 for callers, never 7 and 8.
4. Reversed acyclic contexts form a union SCC without asserting dynamic
   recursion. Recomputing after part selection can split the group again.
5. Zero-count positive-cost edge and all-zero recorded backlink participate
   in topology; changing selected metric or hiding an edge cannot split it.
6. Missing incoming edges, incoming/outgoing disagreement and incomplete
   declarations leave self/edge data unchanged.
7. Event-order remapping works; missing columns remain unavailable, including
   a selection whose event exists only in some parts.
8. Full-path/object collisions and opaque context suffixes remain distinct;
   inline source attribution does not split function topology.
9. Self/group-contribution partition identities, deterministic ordering,
   large/deep graphs, checked arithmetic and zero denominators.
10. Same-file SQLite regression validates self and raw edges with the existing
    references. Add independent cycle-enabled native probes only for any
    specifically claimed KCachegrind display agreement.

Four numerical/topological examples above were checked with a small local
Python arithmetic/reachability probe during this design pass. No production
Rust code changed; no new Rust or native cycle tests are claimed here.

Remaining choices are implementation details: exact public type names, index
storage, pagination, and how much group detail fits the terminal. Giant-cycle
heuristics, part-overlap inference, stack allocation and compatibility modes
are explicitly deferred. The initial cost semantics are specified above.

[part]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/tracedata.cpp#L770-L863
[function]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/tracedata.cpp#L2321-L2396
[setup]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/tracedata.cpp#L2827-L2848
[call]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/tracedata.cpp#L1207-L1223
[dfs]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/tracedata.cpp#L2415-L2514
[config]: https://github.com/KDE/kcachegrind/blob/764dbf2cf5f44e1f982a231e472b9ed2f2b6cc14/libcore/globalconfig.cpp#L20-L27
[manual]: https://valgrind.org/docs/manual/cl-manual.html#cl-manual.cycles
