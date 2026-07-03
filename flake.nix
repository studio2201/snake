{
  description = "Minimalist Nix-built container for Snake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    shared-assets = {
      url = "github:UberMetroid/shared-assets?ref=v3.0.17";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils, shared-assets, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        lib = pkgs.lib;
        rustVersion = pkgs.rust-bin.stable."1.96.0".default.override {
          targets = [ "wasm32-unknown-unknown" ];
        };
        rustPlatform = pkgs.makeRustPlatform {
          rustc = rustVersion;
          cargo = rustVersion;
        };

        # 1. Build the WASM frontend
        frontend = rustPlatform.buildRustPackage {
          pname = "snake-frontend";
          version = "1.0.33";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            # Cargo keys the github-archive tarball of `inputs.shared-assets`
            # to its crate versions (3.0.13 here). The expected hash is the
            # SRI sha256 of the upstream tarball — same value for all three
            # crates because they come from a single tag. Verified against
            # `nix-prefetch-url https://github.com/UberMetroid/shared-assets/archive/refs/tags/v3.0.17.tar.gz`.
            outputHashes = {
              "shared-core-3.0.13" = "sha256-oGbq9cFo2sGByGl3KBYyz6H9OSiVfRrDMHcoV1Kjk9g=";
              "shared-backend-3.0.13" = "sha256-oGbq9cFo2sGByGl3KBYyz6H9OSiVfRrDMHcoV1Kjk9g=";
              "shared-frontend-3.0.13" = "sha256-oGbq9cFo2sGByGl3KBYyz6H9OSiVfRrDMHcoV1Kjk9g=";
            };
          };

          nativeBuildInputs = [
            rustVersion
            pkgs.wasm-bindgen-cli
            pkgs.trunk
          ];

          buildPhase = ''
            export HOME=$TMPDIR
            mkdir -p frontend/Assets/shared-assets
            cp -r ${shared-assets}/* frontend/Assets/shared-assets/
            cd frontend
            trunk build --release
          '';

          installPhase = ''
            mkdir -p $out/dist
            cp -r dist/* $out/dist/
            # NOTE: the published Docker image ships the unoptimised WASM
            # (518 KB). Local development iteration should run
            # frontend/scripts/optimise-wasm.sh after `trunk build --release`
            # to shrink to ~355 KB. Embedding `wasm-opt -Oz` into Nix's
            # installPhase is brittle (the sandboxed cp over a read-only
            # source file requires chmod u+w dance); the local script does
            # the same thing trivially.
          '';
        };

        # 2. Build the Axum backend
        backend = rustPlatform.buildRustPackage {
          pname = "snake-backend";
          version = "1.0.33";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            # Same values as the frontend derivation; see the comment there
            # for why all three map to the same SRI sha256.
            outputHashes = {
              "shared-core-3.0.13" = "sha256-oGbq9cFo2sGByGl3KBYyz6H9OSiVfRrDMHcoV1Kjk9g=";
              "shared-backend-3.0.13" = "sha256-oGbq9cFo2sGByGl3KBYyz6H9OSiVfRrDMHcoV1Kjk9g=";
              "shared-frontend-3.0.13" = "sha256-oGbq9cFo2sGByGl3KBYyz6H9OSiVfRrDMHcoV1Kjk9g=";
            };
          };

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];

          doCheck = true;

          buildPhase = ''
            mkdir -p frontend/Assets/shared-assets
            cp -r ${shared-assets}/* frontend/Assets/shared-assets/
            cargo build --release --bin backend --bin sh
          '';

          installPhase = ''
            mkdir -p $out/bin
            cp target/release/backend $out/bin/snake-backend
            cp target/release/sh $out/bin/sh
          '';
        };

        # 3. Create the layered Docker container image
        dockerImage = pkgs.dockerTools.buildLayeredImage {
          name = "snake-nix";
          tag = "latest";
          
          # Run under the nobody user (UID 65534)
          config = {
            Cmd = [ "${backend}/bin/snake-backend" ];
            WorkingDir = "/app";
            Env = [
              "PORT=4407"
            ];
            ExposedPorts = {
              "4407/tcp" = {};
            };
            User = "65534:65534";
            Healthcheck = {
              Test = [ "CMD-SHELL" "wget -qO- http://localhost:4407/health >/dev/null 2>&1 || exit 1" ];
              Interval = 30000000000;
              Timeout = 10000000000;
              Retries = 3;
              StartPeriod = 60000000000;
            };
          };

          # Create /app directory structure inside the container
          extraCommands = ''
            mkdir -p app/data
            mkdir -p app/frontend
            cp -r ${frontend}/dist app/frontend/dist
                      mkdir -p bin
            cp ${backend}/bin/sh bin/sh
            cp ${backend}/bin/sh bin/bash
'';
        };

      in {
        packages = {
          inherit frontend backend dockerImage;
          default = dockerImage;
        };

        devShells.default = pkgs.mkShell {
          buildInputs = [
            rustVersion
            pkgs.trunk
            pkgs.wasm-bindgen-cli
          ];
        };
      }
    );
}
