{ craneLib, lean-tui }:
craneLib.buildPackage (
  lean-tui.cargoArgs
  // {
    pname = "xtask";
    inherit (lean-tui) cargoArtifacts;
    cargoExtraArgs = "--locked --package xtask";
    doCheck = false;
    meta.mainProgram = "xtask";
  }
)
