{ lib, manifest }:
let
  leanPrismPin = lib.findFirst (
    package: package.name == "LeanPrism"
  ) (throw "LeanPrism missing from ${toString manifest}") (lib.importJSON manifest).packages;
  leanPrismSource = builtins.fetchGit {
    inherit (leanPrismPin) url rev;
    shallow = true;
  };
in
"${leanPrismSource}/protocol-schema.json"
