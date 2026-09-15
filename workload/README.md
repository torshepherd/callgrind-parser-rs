# Workloads for annotator integration tests

The registry in [`default.nix`](default.nix) declares pinned inputs and commands.
The shared runner profiles each variant under the selected Callgrind
configurations; the same raw bytes go to the Rust parser and both annotators.
SQLite currently has two page-cache variants (64 and 4096 pages), each using
all six configurations: **12 profiles and 87 annotation comparisons**, including
the two focused annotation fixtures. Adding a workload does not require a new
profile parser, comparator or workflow implementation.

## Run a workload

With Nix installed, from the repository root:

```bash
# Cached raw corpus and legacy reports; no Rust build required.
./scripts/nix.sh build .#profiles-sqlite -L
# Run comparisons in a writable output directory, retaining failure reports.
./scripts/nix.sh run -L .#integration-sqlite -- "$PWD/results/sqlite"
# The same check in the Nix build sandbox. The raw corpus is reused.
./scripts/nix.sh build .#smoke-sqlite -L
./scripts/nix.sh flake check -L
```

Use an empty output directory; the runner refuses to mix previous results.
`smoke` remains an alias for `smoke-sqlite`, and `fixture` still builds the
SQLite database. CI runs automatically on default-branch pushes. Its workload
matrix selects the registered names, currently `[sqlite]`. The SQLite job also
runs the complete flake check; each selected workload builds its own smoke gate.

Profiles are generated once per Nix input closure and reused by the normal and
sandboxed comparison passes. There is no cross-run custom binary cache yet.
Generated instruction totals are not cross-host goldens: compare the consumers
of the identical profile, never independently profile each annotator's input.

## Add another program

1. Add reproducible input files under `workload/` and a registry entry in
   `default.nix`. Pin the executable through the existing nixpkgs lock (or a
   source/hash declared by that entry). Do not download inputs while profiling.
2. Define named variants with an `argv` array and optional absolute `stdin`
   filename. The runner invokes the executable directly, without shell parsing,
   in a fresh working directory for every configuration. Relative output paths
   such as `output.o` are private to that case. Preparation belongs in a Nix
   derivation, outside the measured command.
3. Select configuration names if the workload does not need all six. Omission
   means every configuration in
   [`configurations.json`](../tests/reference/configurations.json). Optional
   `timeout_seconds` bounds each process (default 300). Long-running workloads
   also need an appropriate CI job timeout and an intentional cost decision.
4. Add its name to the CI `matrix.workload` list. The registry automatically
   provides `profiles-NAME`, `smoke-NAME`, `integration-NAME` and a flake check.
   Run the new integration app and smoke check before claiming support.

For a future Clang workload, the shape could be:

```nix
clang = {
  plan = {
    schema = 1;
    name = "clang";
    configurations = [ "ir-only" "branches-and-jumps" ];
    timeout_seconds = 600;
    variants = [{
      name = "compile";
      argv = [
        "${pkgs.llvmPackages.clang-unwrapped}/bin/clang"
        "-cc1" "-emit-obj" "${./clang/input.cpp}" "-o" "output.o"
      ];
    }];
  };
};
```

This is an extension example, **not an implemented or tested Clang workload**.
Supply a small self-contained translation unit and verify the selected package
contains the actual binary. Nix compiler wrappers and shell launchers otherwise
profile the wrapper process. Clang's
[`-cc1` frontend entry point](https://clang.llvm.org/docs/DriverInternals.html)
lets this example profile compilation directly without a driver spawning a
separate frontend. General process-tree collection is not supported by this
single-profile contract; a future driver workload must explicitly model child
profiles instead of silently missing or combining them.

## Results and comparison scope

`PLAN.json` records schema/version, variants, exact arguments, timeouts and
configuration selection. `SUMMARY.tsv` uses generic `case` and `variant`
columns, plus events, summary and byte counts. The validator checks the exact
case cross-product against the separately supplied expected plan and rejects
missing, extra, duplicate or incomplete cases. It still accepts the old SQLite
manifest columns for historical results. `scripts/run-matrix.sh` retains its
original SQLite CLI and delegates to the same runner; its output must now be empty.

Each case retains raw `.callgrind`, reference `.annotated.txt`, process stdout,
stderr, Valgrind logs, command JSON, stdin copies and generated working files.
`comparisons/` retains both annotators' stdout/stderr and argument lists for
each option set, including mismatches and nonzero exits. CI uploads results
and build/comparison logs on success or failure, retaining artifacts for seven
days. Completed profiles are copied out before building/running Rust checks,
so a Rust build or comparison failure does not discard the corpus. A failure
inside the initial Nix profile-generation derivation retains the CI build log;
its incomplete Nix output is not promised as an artifact.

The comparison checks event order, totals, nonzero function order/costs and
call-tree edges. It normalizes whitespace, percentages, zero display and object
decorations and uses Rust's explicit `--grouping=source`. It does not establish
source-output byte parity, full object identity or a KCachegrind/Qt CI pass.
See [the reference guide](../tests/reference/README.md) for those boundaries.
The existing Python harness is retained for now; a Rust migration is in TODO.md.
