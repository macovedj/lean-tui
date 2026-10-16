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
      recordingManifest = nixpkgs.lib.importJSON ./tests/recordings/lean/lake-manifest.json;
      leanPrismPin = nixpkgs.lib.findFirst (
        package: package.name == "LeanPrism"
      ) (throw "LeanPrism missing from tests/recordings/lean/lake-manifest.json") recordingManifest.packages;
      leanPrismSource = builtins.fetchGit {
        inherit (leanPrismPin) url rev;
        shallow = true;
      };
      recordingScenes = map (nixpkgs.lib.removeSuffix ".rs") (
        builtins.attrNames (builtins.readDir ./tests/recordings/src/scenes)
      );
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
                (lean4-nix.readToolchainFile ./tests/recordings/lean/lean-toolchain)
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
          cargoCommon = {
            src = craneLib.cleanCargoSource ./.;
            pname = "lean-tui-workspace";
            version = "0";
            strictDeps = true;
            LEAN_DAG_SCHEMA = "${leanPrismSource}/protocol-schema.json";
          };
          cargoArtifacts = craneLib.buildDepsOnly cargoCommon;
          buildWorkspaceMember =
            member:
            craneLib.buildPackage (
              cargoCommon
              // {
                inherit cargoArtifacts;
                pname = member;
                cargoExtraArgs = "--package ${member}";
                doCheck = false;
              }
            );
          lean-tui = buildWorkspaceMember "lean-tui";
          lean-tui-recordings = buildWorkspaceMember "lean-tui-recordings";
          clippy = craneLib.cargoClippy (
            cargoCommon
            // {
              inherit cargoArtifacts;
              cargoClippyExtraArgs = "--package lean-tui-recordings";
            }
          );
          lake2nix = pkgs.callPackage lean4-nix.lake { };
          recordingProject = lake2nix.mkPackage {
            name = "Samples";
            src = nixpkgs.lib.fileset.toSource {
              root = ./tests/recordings/lean;
              fileset = nixpkgs.lib.fileset.unions [
                ./tests/recordings/lean/lakefile.toml
                ./tests/recordings/lean/lake-manifest.json
                ./tests/recordings/lean/lean-toolchain
                ./tests/recordings/lean/.helix
                ./tests/recordings/lean/Samples
              ];
            };
            postUnpack = ''
              rm "$sourceRoot/Samples"
              cp -r ${./tests/samples} "$sourceRoot/Samples"
            '';
          };
          recordScene =
            scene:
            pkgs.stdenvNoCC.mkDerivation {
              name = "lean-tui-recording-${scene}";
              src = recordingProject;
              nativeBuildInputs = [
                lean-tui
                lean-tui-recordings
                pkgs.lean.lean-all
                pkgs.helix
                pkgs.asciinema-agg
                pkgs.gifsicle
              ];
              FONTCONFIG_FILE = pkgs.makeFontsConf { fontDirectories = [ pkgs.jetbrains-mono ]; };
              TZDIR = "${pkgs.tzdata}/share/zoneinfo";
              buildPhase = ''
                export HOME=$TMPDIR
                chmod -R u+w .
                lean-tui-recordings record --out $out --font-dir ${pkgs.jetbrains-mono}/share/fonts ${scene}
              '';
              dontInstall = true;
            };
          sceneRecordings = nixpkgs.lib.genAttrs recordingScenes recordScene;
        in
        {
          default = lean-tui;
          inherit
            lean-tui
            lean-tui-recordings
            recordingProject
            clippy
            ;
          demos = pkgs.symlinkJoin {
            name = "lean-tui-recordings";
            paths = builtins.attrValues sceneRecordings;
          };
        }
        // nixpkgs.lib.mapAttrs' (
          scene: recording: nixpkgs.lib.nameValuePair "demo-${scene}" recording
        ) sceneRecordings
      );
      checks = forEachSystem (
        { system, ... }:
        {
          build-lean-tui = self.packages.${system}.lean-tui;
          build-recordings = self.packages.${system}.lean-tui-recordings;
          build-recording-project = self.packages.${system}.recordingProject;
          clippy-recordings = self.packages.${system}.clippy;
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
