# Verified smoke-test result

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
