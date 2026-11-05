{
  pkgs,
  craneLib,
  lean-tui,
  xtask,
}:
let
  generatedDocs = pkgs.lib.fileset.toSource {
    root = ../.;
    fileset = ../docs/keybindings.md;
  };
in
{
  build-lean-tui = lean-tui;
  tests = craneLib.cargoTest (
    lean-tui.cargoArgs
    // {
      inherit (lean-tui) cargoArtifacts;
      doInstallCargoArtifacts = false;
    }
  );
  keybinding-docs = pkgs.runCommand "keybinding-docs" { } ''
    cd ${generatedDocs}
    ${pkgs.lib.getExe xtask} keybinding-docs --check
    touch $out
  '';
}
