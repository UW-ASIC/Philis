{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
          config.allowUnfree = true; # CUDA packages are unfree
        };
        isLinux = pkgs.stdenv.isLinux;
        rust = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
            "clippy"
          ];
        };
      in
      {
        # The packaged CLI. CPU only: the `gpu` feature is opt-in and pulls CUDA/Vulkan,
        # which belongs in the dev shell rather than in something meant to be cached and
        # handed to people who just want to place a netlist.
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "philis";
          version = "0.1.0";
          src = ./.;

          # Vendoring runs `cargo metadata` inside each git checkout, where Substrate2's
          # manifests name a `substrate` registry no config there defines: hand it a
          # cargo that knows the registry (Philis's own build reads .cargo/config.toml).
          cargoDeps = (pkgs.rustPlatform.importCargoLock.override {
            cargo = pkgs.writeShellScriptBin "cargo" ''
              export CARGO_REGISTRIES_SUBSTRATE_INDEX=https://github.com/substrate-labs/crates-index
              exec ${pkgs.cargo}/bin/cargo "$@"
            '';
          }) {
            lockFile = ./Cargo.lock;
            # GPurify is a git dependency, so its source is not content-addressed by
            # crates.io and nix needs the hash spelled out.
            outputHashes = {
              "gpurify-0.1.0" = "sha256-CF7k80QDIDnVXl7vwn+KFGnVP8BAoVKvTuiS6sdVVU4=";
              # Substrate2: one hash covers every crate of the repo.
              "substrate-0.10.3" = "sha256-6tfY+N5bZUAc4uixQHMpiu/mubZCALObk4Llxy0m/RI=";
            };
          };

          # Just the CLI, not the whole workspace (benchmarks, examples).
          cargoBuildFlags = [ "--bin" "philis" ];

          # protobuf: Substrate2's `cache` crate runs protoc in its build script.
          nativeBuildInputs = [ pkgs.pkg-config pkgs.cmake pkgs.protobuf ];
          buildInputs = [ pkgs.highs ];

          # The suite expects fixtures and, in places, a GPU. Signoff for this package is
          # "the binary runs", checked downstream.
          doCheck = false;

          # The decks and sidecars are compiled into the binary (`philis <net.sp> sky130`,
          # backend/verify/src/decks.rs); the sidecars are copied as templates for a
          # user's own `<pdk>.json`.
          postInstall = ''
            mkdir -p $out/share/philis
            cp -r pdks $out/share/philis/
          '';

          meta = {
            description = "Analog place-and-route: SPICE netlist + PDK deck in, GDS out";
            mainProgram = "philis";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rust
            pkgs.maturin
            (pkgs.python312.withPackages (ps: [
              ps.numpy
              ps.scipy
              ps.pip
              ps.gdstk
            ]))
            pkgs.python312
            pkgs.klayout
            pkgs.magic-vlsi
            pkgs.netgen-vlsi
            pkgs.ngspice
            pkgs.curl
            pkgs.cmake
            pkgs.pkg-config
            pkgs.highs
            pkgs.protobuf # protoc, for Substrate2's `cache` build script
          ]
          # CUDA for verify's GPU kernels (feature "gpu", CubeCL). CubeCL needs the
          # toolkit headers (CUDA_PATH) for NVRTC kernel compilation, libnvrtc, and
          # libcuda from the system driver.
          ++ pkgs.lib.optionals isLinux [
            pkgs.cudaPackages.cudatoolkit
          ]
          # GPU visualizer (wgpu + winit): Vulkan loader, Wayland/X11 client libs.
          ++ pkgs.lib.optionals isLinux [
            pkgs.vulkan-loader
            pkgs.vulkan-headers
            pkgs.wayland
            pkgs.wayland-protocols
            pkgs.libxkbcommon
            pkgs.libx11
            pkgs.libxcursor
            pkgs.libxrandr
            pkgs.libxi
          ];

          shellHook =
            let
              volareHash = "1341f54f5ce0c4955326297f235e4ace1eb6d419";
            in
            ''
              export PDK_ROOT="''${PDK_ROOT:-$PWD/.pdk}"
              export PDK="sky130A"

              ${pkgs.lib.optionalString isLinux ''
                # GPU DRC (CubeCL): headers for NVRTC via CUDA_PATH; libcuda comes from
                # the running system's driver dir, libnvrtc from the toolkit.
                export CUDA_PATH="${pkgs.cudaPackages.cudatoolkit}"
                export LD_LIBRARY_PATH="/run/opengl-driver/lib:${pkgs.cudaPackages.cudatoolkit}/lib:${pkgs.vulkan-loader}/lib:${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
              ''}

              if [ ! -d ".venv" ]; then
                echo "==> Creating Python venv..."
                python3 -m venv .venv
              fi
              source .venv/bin/activate

              if [ ! -d "$PDK_ROOT/sky130A/libs.ref" ]; then
                echo "==> Installing sky130A PDK to $PDK_ROOT ..."
                pip install --quiet volare
                volare enable --pdk sky130 --pdk-root "$PDK_ROOT" "${volareHash}"
                if [ -d "$PDK_ROOT/sky130A/libs.ref" ]; then
                  echo "==> sky130A installed successfully"
                else
                  echo "ERROR: sky130A installation failed"
                fi
              fi
            '';
        };
      }
    );
}
