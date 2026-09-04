{
  description = "Callgrind tools workspace and reproducible reference-test matrix";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-26.05";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };

      projectSource = pkgs.lib.cleanSourceWith {
        src = ./.;
        filter = path: type:
          let name = baseNameOf path;
          in !(builtins.elem name [ "result" "results" "target" ]);
      };

      rustWorkspace = pkgs.runCommand "callgrind-tools-workspace-0.0.0" {
        nativeBuildInputs = with pkgs; [
          cargo
          cargo-nextest
          rustc
          stdenv.cc
        ];
      } ''
        cp -R ${projectSource} source
        chmod -R u+w source
        cd source

        export CARGO_HOME="$TMPDIR/cargo-home"
        export CARGO_TARGET_DIR="$TMPDIR/target"

        cargo nextest run --workspace --offline
        cargo build --workspace --release --offline

        mkdir -p "$out/bin"
        for binary in callgrind-annotate callgrind2pprof textgrind webgrind; do
          install -Dm755 "$CARGO_TARGET_DIR/release/$binary" "$out/bin/$binary"
        done
      '';

      fixture = pkgs.runCommand "sqlite-callgrind-fixture.db" {
        nativeBuildInputs = [ pkgs.sqlite ];
      } ''
        export LC_ALL=C
        export TZ=UTC
        sqlite3 "$out" < ${./workload/fixture.sql}
        test "$(sqlite3 "$out" 'PRAGMA integrity_check;')" = "ok"
      '';

      smoke = pkgs.runCommand "sqlite-callgrind-smoke" {
        nativeBuildInputs = with pkgs; [
          bash
          coreutils
          gnugrep
          gnused
          sqlite
          valgrind
        ];
      } ''
        export LC_ALL=C
        export TZ=UTC

        bash ${./scripts/run-matrix.sh} \
          --fixture ${fixture} \
          --workload ${./workload/workload.sql.in} \
          --out "$out"

        test "$(find "$out" -name '*.callgrind' | wc -l)" -eq 12
        test "$(find "$out" -name '*.annotated.txt' | wc -l)" -eq 12
        test "$(($(wc -l < "$out/SUMMARY.tsv") - 1))" -eq 12
      '';
    in
    {
      packages.${system} = {
        inherit fixture rustWorkspace smoke;
        default = rustWorkspace;
      };

      checks.${system} = {
        rust-workspace = rustWorkspace;
        inherit smoke;
      };

      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          cargo
          cargo-nextest
          clippy
          rust-analyzer
          rustc
          rustfmt
          sqlite
          valgrind
        ];

        shellHook = ''
          echo "Rust $(rustc --version | cut -d' ' -f2); SQLite $(sqlite3 --version | cut -d' ' -f1); Valgrind $(valgrind --version | sed 's/^valgrind-//')"
          echo "Fast tests: cargo nextest run --workspace"
          echo "Everything: ./scripts/nix.sh flake check -L"
        '';
      };
    };
}
