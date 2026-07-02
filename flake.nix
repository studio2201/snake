{
  description = "Minimalist Nix-built container for Snake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
    shared-assets = {
      url = "git+file:///home/jeryd/Projects/ubermetroid/shared-assets?ref=v3.0.16";
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
          version = "1.0.11";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "shared-core-3.0.13" = "sha256-5mz8mB/mYogzTy351ZQRWEAvoQ74FCqTMzwmaGOOQGg=";
              "shared-backend-3.0.13" = "sha256-5mz8mB/mYogzTy351ZQRWEAvoQ74FCqTMzwmaGOOQGg=";
              "shared-frontend-3.0.13" = "sha256-5mz8mB/mYogzTy351ZQRWEAvoQ74FCqTMzwmaGOOQGg=";
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
          '';
        };

        # 2. Build the Axum backend
        backend = rustPlatform.buildRustPackage {
          pname = "snake-backend";
          version = "1.0.11";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "shared-core-3.0.13" = "sha256-5mz8mB/mYogzTy351ZQRWEAvoQ74FCqTMzwmaGOOQGg=";
              "shared-backend-3.0.13" = "sha256-5mz8mB/mYogzTy351ZQRWEAvoQ74FCqTMzwmaGOOQGg=";
              "shared-frontend-3.0.13" = "sha256-5mz8mB/mYogzTy351ZQRWEAvoQ74FCqTMzwmaGOOQGg=";
            };
          };

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = [ pkgs.openssl ];

          doCheck = false;

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
