{
  lib,
  craneLib,
  lean-prism,
}:
let
  cargoArgs = {
    src = lib.fileset.toSource {
      root = ../.;
      fileset = lib.fileset.unions [
        (craneLib.fileset.commonCargoSources ../.)
        (lib.fileset.fileFilter (file: file.name == "replay.jsonl") ../demos)
        (lib.fileset.fileFilter (file: file.hasExt "snap") ../src)
      ];
    };
    strictDeps = true;
    LEAN_DAG_SCHEMA = "${lean-prism}/protocol-schema.json";
    meta.mainProgram = "lean-tui";
  };
  cargoArtifacts = craneLib.buildDepsOnly cargoArgs;
in
craneLib.buildPackage (
  cargoArgs
  // {
    inherit cargoArtifacts;
    doCheck = false;
    passthru = { inherit cargoArgs cargoArtifacts; };
  }
)
