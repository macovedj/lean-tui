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
    lean-prism.url = "git+https://codeberg.org/wvhulle/lean-prism?ref=main";
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
      mkApp = program: {
        type = "app";
        program = lib.getExe program;
      };
      perSystem = lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (
        system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ (import rust-overlay) ];
          };
          toolchains = import ./nix/toolchains.nix { inherit pkgs crane; };
          lean-tui = import ./nix/package.nix {
            inherit (pkgs) lib;
            inherit (toolchains) craneLib;
            inherit lean-prism;
          };
          xtask = import ./nix/xtask.nix {
            inherit (toolchains) craneLib;
            inherit lean-tui;
          };
          xtaskTool = import ./nix/xtask-tool.nix { inherit pkgs xtask; };
        in
        {
          inherit pkgs toolchains lean-tui;
          checks = import ./nix/checks.nix {
            inherit (toolchains) craneLib;
            inherit pkgs lean-tui xtask;
          };
          release = import ./nix/release.nix {
            inherit (toolchains) nightlyWithTargets;
            inherit pkgs crane lean-tui;
          };
          ciTaskApps = import ./nix/ci-tasks.nix {
            inherit pkgs mkApp;
            cargoToolchain = toolchains.nightlyWithTargets [ ];
            preCommitInputs = [ toolchains.rustfmtToolchain ];
          };
          recordings = import ./demos {
            inherit
              pkgs
              lean4-nix
              xtaskTool
              lean-tui
              ;
            lean-prism-server = lean-prism.packages.${system}.lean-prism;
          };
        }
      );
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
          build-dependencies = system.lean-tui.cargoArtifacts;
          x86_64-linux-musl = system.release.muslPackages.x86_64;
          aarch64-linux-musl = system.release.muslPackages.aarch64;
        }
        // lib.mapAttrs' (
          scene: recording: lib.nameValuePair "recording-${scene}" recording
        ) system.recordings.scenes
      ) perSystem;
      checks = builtins.mapAttrs (_: system: system.checks) perSystem;
      apps = builtins.mapAttrs (
        _: system:
        system.ciTaskApps
        // {
          publish-screenshots = mkApp system.recordings.publish-screenshots;
        }
      ) perSystem;
      devShells = builtins.mapAttrs (_: system: {
        default = system.toolchains.devCrane.devShell {
          packages = [
            system.recordings.lean-toolchain
            system.pkgs.release-plz
            system.pkgs.convco
            system.pkgs.git-cliff
          ];
          MATHLIB_NO_CACHE_ON_UPDATE = "1";
          LEAN_DAG_SCHEMA = "${lean-prism}/protocol-schema.json";
        };
      }) perSystem;
    };
}
