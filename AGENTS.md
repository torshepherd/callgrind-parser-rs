# Development instructions

Nix supplies the Rust toolchain and all native project dependencies, including
SQLite and Valgrind. Do not install or select ad-hoc system versions.

Run these commands from the repository root:

- Fast unit tests: `./scripts/nix.sh develop -c cargo nextest run --workspace`
- Standard Cargo fallback: `./scripts/nix.sh develop -c cargo test --workspace`
- Formatting: `./scripts/nix.sh develop -c cargo fmt --all --check`
- Lints: `./scripts/nix.sh develop -c cargo clippy --workspace --all-targets --all-features -- -D warnings`
- Reference integration matrix: `./scripts/nix.sh build .#smoke -L`
- Complete repository check: `./scripts/nix.sh flake check -L`
- Interactive environment: `./scripts/nix.sh develop`

The integration matrix must produce and validate twelve Callgrind profiles and
twelve reports from the reference `callgrind_annotate`. Raw instruction/event
totals are not golden values because CPU dispatch can differ across x86_64
hosts. Future differential tests must run the Rust and Valgrind annotators on
the exact same generated profile.

Keep parsing and data-model code in `callgrind-parser`; the CLI and UI crates
should consume that library rather than inventing their own profile readers.
