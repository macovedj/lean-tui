{
  nixConfig = {
    extra-substituters = [ "https://wvhulle.cachix.org" ];
    extra-trusted-public-keys = [ "wvhulle.cachix.org-1:heXx8DZMiRsKUx6l1TxNoF+Nmtmz66QEdsonQzc1ir0=" ];
  };

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    lean4-nix = {
      url = "github:lenianiva/lean4-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      rust-overlay,
      nixpkgs,
      crane,
      lean4-nix,
    }:
    let
      leanPrismSource = import ./nix/lean-prism-source.nix {
        inherit (nixpkgs) lib;
        manifest = ./tests/recordings/lake-manifest.json;
      };
      forEachSystem =
        f:
        nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (
          system:
          f rec {
            inherit system;
            pkgs = import nixpkgs {
              inherit system;
              overlays = [
                (import rust-overlay)
                (lean4-nix.readToolchainFile ./tests/recordings/lean-toolchain)
              ];
            };
            craneLib = (crane.mkLib pkgs).overrideToolchain (
              pkgs.rust-bin.selectLatestNightlyWith (
                t:
                t.default.override {
                  extensions = [ "rust-src" ];
                }
              )
            );
          }
        );
    in
    {
      packages = forEachSystem (
        { pkgs, craneLib, ... }:
        let
          lean-tui = import ./nix/lean-tui.nix { inherit craneLib leanPrismSource; };
          recordings = import ./tests/recordings {
            inherit (nixpkgs) lib;
            inherit pkgs lean4-nix lean-tui;
          };
        in
        {
          default = lean-tui;
          inherit lean-tui;
          inherit (recordings) record-scene;
          recording-project = recordings.project;
          recordings = recordings.all;
        }
        // nixpkgs.lib.mapAttrs' (
          scene: recording: nixpkgs.lib.nameValuePair "recording-${scene}" recording
        ) recordings.scenes
      );
      checks = forEachSystem (
        { system, ... }:
        {
          build-lean-tui = self.packages.${system}.lean-tui;
          build-recording-project = self.packages.${system}.recording-project;
        }
      );
      devShells = forEachSystem (
        { pkgs, craneLib, ... }:
        {
          default = craneLib.devShell {
            packages = [ pkgs.elan ];
            MATHLIB_NO_CACHE_ON_UPDATE = "1";
          };
        }
      );
    };
}
