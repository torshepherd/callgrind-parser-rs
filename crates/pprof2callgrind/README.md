# pprof2callgrind

> **Experimental — documentation status:** This crate is experimental. Its
> documentation is fully LLM-generated and may contain errors or outdated claims.
> The documentation will receive a review and cleanup pass before the 1.0 release.

Convert pprof raw protobuf or gzip into exact-integer Callgrind, retaining every
sample-value column. No Go, protoc, system zlib or symbolizer is needed.

```console
./scripts/cargo.sh run -p pprof2callgrind --locked --offline -- cpu.pb.gz -o cpu.callgrind
./scripts/cargo.sh run -p pprof2callgrind --locked --offline -- --mode tree cpu.pb.gz -o tree.callgrind
```

Input `-` reads stdin. Omitted output or `-o -` writes stdout. Named output
must not already exist. Conversion is validated before creating it; an output
I/O failure can still leave a partial new file. Warnings go only to stderr.

## Semantics

- **Graph (default):** exclusive cost goes only to the sampled leaf. Each
  distinct ordered frame edge receives a sample's full vector once, including
  recursive self edges. This aggregate graph cannot recover full stacks.
- **Tree:** each root-first stack prefix has a separate context identity.
  Recursive occurrences stay distinct; edges carry exact sampled subtree
  cost. Samples ending at intermediate nodes retain their exclusive cost.
  Labels are still aggregated: this is not lossless pprof serialization.
- Inline chains expand in pprof's leaf-first order. Unsymbolized locations
  remain identifiable; no binary metadata or call counts are invented.
- All `E0`, `E1`, ... columns retain their original integer units and order.
  `event:` descriptions give the original pprof type and unit. No float scaling,
  mean, period multiplication, filtering, pruning or default-column selection.
- Call counts are **unknown**, written as `calls=0`. These are sampled weights,
  not invocation counts. Legacy Perl `callgrind_annotate` misclassifies these
  edges; use the Rust annotator or KCachegrind. Never substitute fake counts.
- Negative values/source lines, nonzero samples with empty stacks and `u64`
  aggregate overflow fail. Zero-valued samples are skipped. Empty profiles
  with declared sample types produce zero totals.

## Identity and representation

Graph frame identity is `(mapping ID, function ID, address, line)`; duplicate
location IDs describing the same known frame merge. Unsymbolized frames also
retain their location ID. Function names gain `[pprof:fN:mN]`, unknown frames
`[pprof:lN:mN]`, and tree contexts `[ctx:N]`. Distinct definitions sharing names
or paths do not merge. Ordering is deterministic for fixed IDs and independent
of sample order.

Object/file/function strings use injective UTF-8 percent encoding for `%`,
controls, boundary spaces, other whitespace and leading `(`. Empty strings
are `%EMPTY`; interior ASCII spaces and ordinary Unicode remain readable.
Readers display encoded names, not automatically decoded source paths.
Absolute positions and explicit `cob`/`cfl`/`cfn` avoid relative-target state
bugs and cross-object misattribution.

PCs retain pprof semantics, not Valgrind relocation semantics. Mapping ranges,
offsets/build IDs, function start lines, separate system names, columns, folded
flags, labels, comments, timing and periods do not survive as structured
Callgrind metadata. Stored frame-filter directives are not applied. Unknown
protobuf fields are not retained by the shared reader. Warnings state the
losses; this is not an inverse of pprof's exporter.

Defaults: 256 MiB compressed and decoded input limits, one million unique frames
or tree contexts, expanded depth 16,384. Override with `--max-input-bytes`,
`--max-nodes`, `--max-depth`. These bound specific structures, not total memory;
protobuf objects, edge tables and indexes need additional memory.

## Validation

Rust tests cover I/O, gzip integrity, identities, exact integers, all columns,
recursion, inlining, context ambiguity, limits and CLI failures. A separate
Actions job uses pinned upstream pprof to generate 12 fixtures and cross-read
Rust protobuf/gzip, then compares 22 converted files with pinned, unmodified
KCachegrind libcore. Raw totals/self/edge/source-line costs and identities must
agree; GUI inclusive/cycle heuristics are excluded. Known input stacks also
constrain exact self costs, edge weights and tree paths independently.
See [the gate](../../tests/reference/pprof-audit/README.md).

[`callgrind2pprof`](../callgrind2pprof/README.md) provides separate flat
exclusive-cost reverse conversion; it does not reconstruct caller stacks.
Shared `pprof-profile` and `callgrind-writer` are reusable building blocks.
