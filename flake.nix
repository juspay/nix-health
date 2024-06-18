{
  nixConfig = {
    # https://garnix.io/docs/caching
    extra-substituters = "https://cache.garnix.io";
    extra-trusted-public-keys = "cache.garnix.io:CTFPyKSLcx5RMJKfLo5EEPUObbA78b0YQ2DTCJXqr9g=";
  };
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    systems.url = "github:nix-systems/default";
    rust-flake.url = "github:juspay/rust-flake";
    rust-flake.inputs.nixpkgs.follows = "nixpkgs";

    # Dev tools
    treefmt-nix.url = "github:numtide/treefmt-nix";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      systems = import inputs.systems;
      imports = [
        inputs.treefmt-nix.flakeModule
        inputs.rust-flake.flakeModules.default
        inputs.rust-flake.flakeModules.nixpkgs
        ./module/flake-module.nix
      ];
      perSystem = { config, self', pkgs, lib, system, ... }: {
        rust-project.crane.args = {
          buildInputs = lib.optionals pkgs.stdenv.isDarwin (
            with pkgs.darwin.apple_sdk.frameworks; [
              IOKit
              # apple_sdk refers to SDK version 10.12. To compile for `x86_64-darwin` we need 11.0
              # see: https://github.com/NixOS/nixpkgs/pull/261683#issuecomment-1772935802
              pkgs.darwin.apple_sdk_11_0.frameworks.CoreFoundation
            ]
          );
          nativeBuildInputs = with pkgs; [
            nix # Tests need nix cli
          ];
        } // lib.optionalAttrs pkgs.stdenv.isLinux {
          CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
          CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
        };

        # Add your auto-formatters here.
        # cf. https://numtide.github.io/treefmt/
        treefmt.config = {
          projectRootFile = "flake.nix";
          programs = {
            nixpkgs-fmt.enable = true;
            rustfmt.enable = true;
          };
        };

        devShells.default = pkgs.mkShell {
          inputsFrom = [
            self'.devShells.nix_health
            config.treefmt.build.devShell
            config.nix-health.outputs.devShell
          ];
        };
        packages.default = self'.packages.nix_health.overrideAttrs ({
          meta.mainProgram = "nix-health";
        });
      };

      flake = {
        flakeModule = ./module/flake-module.nix;
        nix-health.default = {
          nix-version.min-required = "2.17.0";
        };
      };
    };
}
