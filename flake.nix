{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      rust-overlay,
      nixpkgs,
      crane,
    }:
    let
      forEachSystem =
        f:
        nixpkgs.lib.genAttrs [ "x86_64-linux" "aarch64-linux" ] (
          system:
          f rec {
            inherit system;
            pkgs = import nixpkgs {
              inherit system;
              overlays = [ (import rust-overlay) ];
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
        { craneLib, ... }:
        {
          default = craneLib.buildPackage (craneLib.cleanCargoSource ./.);
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
