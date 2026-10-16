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
      rust-overlay,
      nixpkgs,
      crane,
      lean4-nix,
      ...
    }:
    let
      protocolSchema = import ./protocol-schema.nix {
        inherit (nixpkgs) lib;
        manifest = ./tests/recordings/lake-manifest.json;
      };
      perSystem = nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [
              (import rust-overlay)
              (lean4-nix.readToolchainFile ./tests/recordings/lean-toolchain)
            ];
          };
          craneLib = (crane.mkLib pkgs).overrideToolchain (
            pkgs.rust-bin.selectLatestNightlyWith (
              toolchain: toolchain.default.override { extensions = [ "rust-src" ]; }
            )
          );
          lean-tui = import ./package.nix { inherit craneLib protocolSchema; };
          recordings = import ./tests/recordings {
            inherit (nixpkgs) lib;
            inherit pkgs lean4-nix lean-tui;
          };
        in
        {
          inherit pkgs craneLib lean-tui recordings;
          checks = import ./checks.nix {
            inherit craneLib lean-tui;
            recordingProject = recordings.project;
          };
          release = import ./release { inherit pkgs crane lean-tui; };
        }
      );
      mkApp = program: {
        type = "app";
        program = nixpkgs.lib.getExe program;
      };
    in
    {
      packages = builtins.mapAttrs (
        _: system:
        {
          default = system.lean-tui;
          inherit (system) lean-tui;
          inherit (system.recordings) record-scene;
          recording-project = system.recordings.project;
          recordings = system.recordings.all;
          inherit (system.release) dist;
          x86_64-linux-musl = system.release.muslPackages.x86_64;
          aarch64-linux-musl = system.release.muslPackages.aarch64;
        }
        // nixpkgs.lib.mapAttrs' (
          scene: recording: nixpkgs.lib.nameValuePair "recording-${scene}" recording
        ) system.recordings.scenes
      ) perSystem;
      checks = builtins.mapAttrs (_: system: system.checks) perSystem;
      apps = builtins.mapAttrs (_: system: {
        release = mkApp system.release.release;
        release-pr = mkApp system.release.release-pr;
        cached-build = mkApp system.release.cached-build;
        check-commits = mkApp system.release.check-commits;
      }) perSystem;
      devShells = builtins.mapAttrs (_: system: {
        default = system.craneLib.devShell {
          packages = [
            system.pkgs.elan
            system.pkgs.release-plz
            system.pkgs.convco
            system.pkgs.git-cliff
          ];
          MATHLIB_NO_CACHE_ON_UPDATE = "1";
        };
      }) perSystem;
    };
}
