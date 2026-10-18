{
  craneLib,
  rustfmtCrane,
  lean-tui,
}:
{
  build-lean-tui = lean-tui;
  tests = craneLib.cargoTest (
    lean-tui.cargoArgs
    // {
      inherit (lean-tui) cargoArtifacts;
      doInstallCargoArtifacts = false;
    }
  );
  fmt = rustfmtCrane.cargoFmt { inherit (lean-tui.cargoArgs) src; };
}
