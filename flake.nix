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

          cargoLock = {
            lockFile = ./Cargo.lock;
            # GPurify is a git dependency, so its source is not content-addressed by
            # crates.io and nix needs the hash spelled out.
            outputHashes = {
              "gdsverify-0.1.0" = "sha256-0Hzr+HkTTFh3piymqpNBewwSidySjYRHIk6iaovmKkk=";
            };
          };

          # Just the CLI. Building the whole workspace would drag in tools/visualizer,
          # which is wgpu + winit and wants a display stack this package has no use for.
          cargoBuildFlags = [ "--bin" "philis" ];

          nativeBuildInputs = [ pkgs.pkg-config pkgs.cmake ];
          buildInputs = [ pkgs.highs ];

          # The suite expects fixtures and, in places, a GPU. Signoff for this package is
          # "the binary runs", checked downstream.
          doCheck = false;

          # The rule decks travel with the binary: `philis run` needs one, and a CLI that
          # cannot find its own PDK deck is not much of a package.
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
            pkgs.ngspice
            pkgs.curl
            pkgs.cmake
            pkgs.pkg-config
            pkgs.highs
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
              # open_pdks commit the PDK build is pinned to. Both ciel and volare
              # address releases by this hash, so the pin survives the migration.
              pdkHash = "1341f54f5ce0c4955326297f235e4ace1eb6d419";
            in
            ''
              export PDK_ROOT="''${PDK_ROOT:-$PWD/.pdk}"
              export PDK="sky130A"
              # ngspice resolves `nfet_01v8` & co. from here when the flow extracts
              # a DC operating point (per-device power for the thermal solver,
              # bias current for electromigration).
              export SKY130_MODELS="$PDK_ROOT/$PDK/libs.tech/ngspice"
              export SPICE_LIB_DIR="''${SPICE_LIB_DIR:-$SKY130_MODELS}"

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

              # Both the layout libs (libs.ref) and the ngspice models
              # (libs.tech/ngspice) are required — the models are what let the flow
              # run a real `.op`, so a PDK missing them is not usable here.
              if [ ! -d "$PDK_ROOT/$PDK/libs.ref" ] || [ ! -d "$SKY130_MODELS" ]; then
                echo "==> Installing $PDK to $PDK_ROOT ..."
                # ciel is the maintained successor to volare (same release hashes);
                # fall back to volare where ciel is unavailable.
                if pip install --quiet ciel 2>/dev/null && command -v ciel >/dev/null; then
                  ciel enable --pdk sky130 --pdk-root "$PDK_ROOT" "${pdkHash}" \
                    || echo "WARN: ciel enable failed"
                else
                  echo "==> ciel unavailable, falling back to volare"
                  pip install --quiet volare
                  volare enable --pdk sky130 --pdk-root "$PDK_ROOT" "${pdkHash}" \
                    || echo "WARN: volare enable failed"
                fi

                if [ -d "$PDK_ROOT/$PDK/libs.ref" ] && [ -d "$SKY130_MODELS" ]; then
                  echo "==> $PDK installed (layout libs + ngspice models)"
                else
                  echo "ERROR: $PDK installation incomplete —"
                  echo "       libs.ref:  $( [ -d "$PDK_ROOT/$PDK/libs.ref" ] && echo ok || echo MISSING )"
                  echo "       ngspice:   $( [ -d "$SKY130_MODELS" ] && echo ok || echo MISSING )"
                fi
              fi
            '';
        };
      }
    );
}
