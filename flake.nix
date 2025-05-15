{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.11";
    flake-utils.url = "github:numtide/flake-utils";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
    fenix,
    crane,
  }:
    flake-utils.lib.eachDefaultSystem
    (
      system: let
        overlays = [fenix.overlays.default];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        releaseTarget = "x86_64-unknown-linux-musl";
        craneLib = (crane.mkLib pkgs).overrideToolchain (pkgs.fenix.combine [
          pkgs.fenix.minimal.rustc
          pkgs.fenix.minimal.cargo
          pkgs.fenix.targets.${releaseTarget}.latest.rust-std
        ]);
        commonArgs = {
          src = pkgs.lib.sources.cleanSourceWith {
            src = ./.;
            filter = orig_path: type:
              baseNameOf orig_path
              == "initial_steam_games.json"
              || craneLib.filterCargoSources orig_path type;
            name = "source";
          };
          strictDeps = true;
        };
        crate = craneLib.buildPackage (commonArgs
          // {
            CARGO_BUILD_TARGET = releaseTarget;
            CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
            cargoArtifacts = craneLib.buildDepsOnly commonArgs;
          });
      in
        with pkgs; {
          formatter = alejandra;
          devShells.default = mkShell rec {
            buildInputs = [
              pkgs.fenix.stable.completeToolchain
              pnpm
              nodejs_22
            ];
          };

          packages = with pkgs; rec {
            default = crate;
            pushDockerImage = writeShellScriptBin "push-docker-image" ''
              sudo ${docker}/bin/docker image load -i ${dockerImage}
              sudo ${docker}/bin/docker push kokuzo.tailc38f.ts.net/personal-api:latest
            '';
            dockerImage = pkgs.dockerTools.buildLayeredImage {
              name = "kokuzo.tailc38f.ts.net/personal-api";
              tag = "latest";
              config = {
                Entrypoint = ["${default}/bin/personal-api"];
                ExposedPorts = {
                  "3000 su/tcp" = {};
                };
              };
            };
          };
        }
    );
}
