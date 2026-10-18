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
    lean-prism = {
      url = "git+https://codeberg.org/wvhulle/lean-prism?ref=lean-v4.33";
      flake = false;
    };
  };

  outputs =
    {
      nixpkgs,
      crane,
      rust-overlay,
      lean4-nix,
      lean-prism,
      ...
    }:
    let
      inherit (nixpkgs) lib;
      perSystem = lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          toolchains = import ./nix/toolchains.nix { inherit pkgs crane; };
          writeNuTool = import ./nix/nu-tool.nix { inherit pkgs; };
          lean-tui = import ./nix/package.nix {
            inherit (toolchains) craneLib;
            inherit lean-prism;
          };
        in
        {
          inherit pkgs toolchains lean-tui;
          checks = import ./nix/checks.nix {
            inherit (toolchains) craneLib rustfmtCrane;
            inherit lean-tui;
          };
          release = import ./nix/release.nix {
            inherit (toolchains) nightlyWithTargets;
            inherit
              pkgs
              crane
              writeNuTool
              lean-tui
              ;
          };
          ci = import ./nix/ci.nix { inherit pkgs writeNuTool; };
          recordings = import ./recordings {
            inherit
              pkgs
              lean4-nix
              writeNuTool
              lean-tui
              ;
          };
        }
      );
      mkApp = program: {
        type = "app";
        program = lib.getExe program;
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
        // lib.mapAttrs' (
          scene: recording: lib.nameValuePair "recording-${scene}" recording
        ) system.recordings.scenes
      ) perSystem;
      checks = builtins.mapAttrs (_: system: system.checks) perSystem;
      apps = builtins.mapAttrs (_: system: {
        release = mkApp system.release.release;
        release-pr = mkApp system.release.release-pr;
        upload-release-assets = mkApp system.release.upload-release-assets;
        cached-build = mkApp system.ci.cached-build;
        check-commits = mkApp system.ci.check-commits;
        publish-screenshots = mkApp system.recordings.publish-screenshots;
      }) perSystem;
      devShells = builtins.mapAttrs (_: system: {
        default = system.toolchains.devCrane.devShell {
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
