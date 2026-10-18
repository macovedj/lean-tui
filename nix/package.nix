{ craneLib, lean-prism }:
let
  cargoArgs = {
    src = craneLib.cleanCargoSource ../.;
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
