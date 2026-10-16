{ craneLib, leanPrismSource }:
craneLib.buildPackage {
  src = craneLib.cleanCargoSource ../.;
  LEAN_DAG_SCHEMA = "${leanPrismSource}/protocol-schema.json";
}
