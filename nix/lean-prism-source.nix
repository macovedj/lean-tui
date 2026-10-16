{ lib, manifest }:
let
  pin = lib.findFirst (
    package: package.name == "LeanPrism"
  ) (throw "LeanPrism missing from ${toString manifest}") (lib.importJSON manifest).packages;
in
builtins.fetchGit {
  inherit (pin) url rev;
  shallow = true;
}
