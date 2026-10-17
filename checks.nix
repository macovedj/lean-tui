{
  craneLib,
  rustfmtCrane,
  lean-tui,
  recordingProject,
}:
{
  build-lean-tui = lean-tui;
  build-recording-project = recordingProject;
  tests = craneLib.cargoTest (
    lean-tui.cargoArgs
    // {
      inherit (lean-tui) cargoArtifacts;
      doInstallCargoArtifacts = false;
    }
  );
  fmt = rustfmtCrane.cargoFmt { inherit (lean-tui.cargoArgs) src; };
}
