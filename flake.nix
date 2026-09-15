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

      workloads = import ./workload { inherit pkgs; };
      integrations = pkgs.lib.mapAttrs (name: workload:
        let
          plan = pkgs.writeText "${name}-plan.json" (builtins.toJSON workload.plan);
          profiles = pkgs.runCommand "callgrind-${name}-profiles" {
            nativeBuildInputs = [ pkgs.python3 pkgs.valgrind ];
          } ''
            export LC_ALL=C TZ=UTC
            python3 ${./tests/reference}/run_workload.py --plan ${plan} --out "$out"
          '';
          runner = pkgs.writeShellApplication {
            name = "integration-${name}";
            runtimeInputs = [ pkgs.coreutils pkgs.python3 ];
            text = ''
              export LC_ALL=C TZ=UTC
              if [[ $# != 1 ]]; then
                echo "Usage: integration-${name} EMPTY_OUTPUT_DIRECTORY" >&2
                exit 2
              fi
              mkdir -p "$1"
              output="$(realpath "$1")"
              if [[ -n "$(ls -A "$output")" ]]; then
                echo "Output directory must be empty: $output" >&2
                exit 2
              fi
              cp -R ${profiles}/. "$output/"
              chmod -R u+w "$output"
              python3 -m unittest discover -s ${./tests/reference} -v
              python3 ${./tests/reference}/check_matrix.py "$output" \
                --plan ${plan} --inspect ${rustWorkspace}/libexec/inspect
              python3 ${./tests/reference}/check_annotate.py \
                --rust ${rustWorkspace}/bin/callgrind-annotate \
                --reference ${pkgs.valgrind}/bin/callgrind_annotate \
                --parser-fixtures ${./crates/callgrind-parser/tests/fixtures} \
                --matrix "$output" --plan ${plan} --artifacts "$output/comparisons"
            '';
          };
          smoke = pkgs.runCommand "callgrind-${name}-integration" {} ''
            ${runner}/bin/integration-${name} "$out"
          '';
        in { inherit plan profiles runner smoke; }
      ) workloads;
      namedPackages = pkgs.lib.foldlAttrs (acc: name: suite: acc // {
        "profiles-${name}" = suite.profiles;
        "smoke-${name}" = suite.smoke;
        "integration-${name}" = suite.runner;
      }) {} integrations;
    in
    {
      packages.${system} = {
        inherit rustWorkspace;
        fixture = workloads.sqlite.fixture;
        smoke = integrations.sqlite.smoke;
        default = rustWorkspace;
      } // namedPackages;

      apps.${system} = pkgs.lib.mapAttrs' (name: suite:
        pkgs.lib.nameValuePair "integration-${name}" {
          type = "app";
          program = "${suite.runner}/bin/integration-${name}";
        }
      ) integrations;

      checks.${system} = {
        rust-workspace = rustWorkspace;
      } // pkgs.lib.mapAttrs' (name: suite:
        pkgs.lib.nameValuePair "smoke-${name}" suite.smoke
      ) integrations;

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
