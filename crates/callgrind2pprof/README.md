# callgrind2pprof

Export exact exclusive Callgrind costs as gzip-compressed pprof protobuf.
Every stored event column is retained. The production parser owns all grammar;
the shared `pprof-profile` crate owns protobuf validation and gzip I/O.

```console
./scripts/cargo.sh run -p callgrind2pprof --locked --offline -- profile.callgrind -o profile.pb.gz
./scripts/cargo.sh run -p callgrind2pprof --locked --offline -- combined.callgrind --part 1 -o part1.pb.gz
./scripts/cargo.sh run -p callgrind2pprof --locked --offline -- profile.callgrind --unit sysTime=microseconds -o profile.pb.gz
```

`--part` is a zero-based index, required for multipart input. Parts are never
silently merged. Input `-` reads stdin; output omitted or `-o -` writes gzip
to stdout. Named output refuses overwrite. Parsing and conversion finish before
the output file is created. I/O failure can leave a partial new file, returns
failure, and is never hidden by buffered-writer destruction. Warnings use stderr.

## Costs and limits

Only self-cost rows become samples, each containing **one location**. Repeated
identical locations sum with checked arithmetic. Call-edge inclusive costs,
call counts and jumps are excluded. Zero rows are omitted. Declared `totals`
must agree with summed self costs; `summary` is retained as a comment but does
not contribute to sample values. No means, filtering, scaling or period
multiplication is applied. Derived-event formulas are not evaluated.

All values, coalesced location sums and per-event profile totals must fit
`i64::MAX`; otherwise conversion fails without wrapping, clamping or rounding.
The profile-total limit also prevents downstream signed aggregation overflow.
Source lines must fit signed pprof line fields. Instruction/basic-block labels
retain the full unsigned 64-bit range. Empty/all-zero profiles retain their
event types and have no samples.

Defaults: maximum input text 256 MiB and one million nonzero unique locations.
Override with `--max-input-bytes` and `--max-locations`. The CLI currently reads
the input and collects the owned parser model; these limits are not a total
memory guarantee. No files named inside the profile are opened.

## Units

Types keep the exact event names and order. The first stored column is the
explicit pprof default; pprof users can select another with `-sample_index`.

| Events | Default pprof unit |
| --- | --- |
| Ir, Dr, Dw; I1mr, D1mr, D1mw; ILmr, DLmr, DLmw; I2mr, D2mr, D2mw; Bc, Bcm, Bi, Bim; sysCount | `count` |
| sysTime, cache-use metrics, custom or unrecognized events | `callgrind_raw` |

`sysTime` is producer/mode dependent, so no time unit is guessed from its name.
`--unit EVENT=UNIT` declares an explicitly known unit without changing values.
Unknown event names, duplicate overrides, empty units and control characters
are errors. Raw units generate a warning. Descriptive event headers are copied
to comments, not interpreted as a machine-readable unit schema.

## Identities, positions and losses

A pprof function is keyed by the original object, defining file, function name
and attributed source file. Its display name gains a deterministic
`[callgrind:fN]` suffix so pprof cannot silently merge same-named definitions.
Its filename is the attributed source file. An inline-attributed source region
can therefore appear as a separate qualified function entry; we do not invent
an inline chain. Original defining identity remains available in sample labels.

Location identity adds every declared instruction, basic-block and line
position, including zero. Sorted identities make output deterministic for
equivalent record orderings. Missing names and literal `???` stay distinct.

Sample labels preserve the original data:

| Label | Representation |
| --- | --- |
| `callgrind.object`, `.function`, `.defining_file`, `.source_file` | `v:` followed by the exact original string; absent values omit the label |
| `callgrind.instruction`, `.basic_block`, `.line` | Unsigned decimal string, when that position column exists |

The `v:` prefix makes empty strings representable without confusing pprof's
string-label/numeric-label encoding. These labels are strings, not numeric
metrics. Native line fields hold the source line (zero if unknown/absent).

Callgrind PCs are not necessarily runtime virtual addresses. Consequently
pprof location addresses are zero and mappings are absent; original PCs remain
labels. No load ranges, offsets, build IDs, function start lines, system names,
sample timing, periods or full stacks are invented. Selected part index and
available process/thread/part/command metadata are recorded in comments.
Other producer descriptions/extensions are not serialized.

This is a **flat profile**, not an inverse of `pprof2callgrind`. Pprof computes
cumulative values from these single-location samples, so they equal self costs;
caller attribution/flamegraph depth is deliberately absent. Aggregate Callgrind
edges cannot generally reconstruct original stack populations. See the
[source audit](../../docs/PPROF-CALLGRIND-AUDIT.md).

## Verification

Rust regressions cover exact large values/all columns, signed-range failures,
repeated locations, object/path collisions, inline attribution, original PCs,
unknown names, units, multipart layouts, totals, output failure and CLI behavior.
The separate native CI gate uses pinned upstream Go `ParseData`/`CheckValid`
and actual `pprof -top` reports. It compares per-event sums and exclusive
function/source-line rows against the production parser on seven focused files,
22 forward-converter outputs and the **same 12 SQLite profiles** from the
successful Nix integration job: 41 files / 43 parts. Integer readback is the
cost oracle; pprof's formatted percentages/rounded display are not.
