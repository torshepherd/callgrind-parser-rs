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
          in pkgs.lib.cleanSourceFilter path type
            && !(builtins.elem name [ ".dev" "result" "results" "target" ]);
      };

      rustWorkspace = pkgs.rustPlatform.buildRustPackage {
        pname = "callgrind-tools-workspace";
        version = "0.0.0";
        src = projectSource;
        # Fetch dependencies by Cargo.lock checksums before the offline build.
        cargoLock.lockFile = ./Cargo.lock;
        cargoBuildFlags = [ "--workspace" "--bins" "--examples" ];
        cargoTestFlags = [ "--workspace" ];
        useNextest = true;
        postCheck = ''
          cargo test --workspace --doc --locked --offline
        '';
        postInstall = ''
          mkdir -p "$out/libexec"
          install -m755 "$tmpDir/examples/inspect" "$tmpDir/examples/reference_export" "$out/libexec/"
        '';
      };

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
          python3
        ];
      } ''
        export LC_ALL=C
        export TZ=UTC

        bash ${./scripts/run-matrix.sh} \
          --fixture ${fixture} \
          --workload ${./workload/workload.sql.in} \
          --out "$out"

        python3 -m unittest discover -s ${./tests/reference} -v
        python3 ${./tests/reference}/check_matrix.py "$out" \
          --inspect ${rustWorkspace}/libexec/inspect
        python3 ${./tests/reference}/check_annotate.py \
          --rust ${rustWorkspace}/bin/callgrind-annotate \
          --reference ${pkgs.valgrind}/bin/callgrind_annotate \
          --parser-fixtures ${./crates/callgrind-parser/tests/fixtures} \
          --matrix "$out"
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
