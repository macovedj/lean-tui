{ pkgs, crane }:
let
  craneWithNightly =
    selectComponents:
    (crane.mkLib pkgs).overrideToolchain (pkgs.rust-bin.selectLatestNightlyWith selectComponents);
in
{
  craneLib = craneWithNightly (toolchain: toolchain.minimal);
  rustfmtToolchain = pkgs.rust-bin.selectLatestNightlyWith (
    toolchain: toolchain.minimal.override { extensions = [ "rustfmt" ]; }
  );
  devCrane = craneWithNightly (
    toolchain: toolchain.default.override { extensions = [ "rust-src" ]; }
  );
  nightlyWithTargets =
    targets:
    pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.minimal.override { inherit targets; });
}
