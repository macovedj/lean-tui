{
  lib,
  pkgs,
  lean4-nix,
  lean-tui,
}:
let
  lake2nix = pkgs.callPackage lean4-nix.lake { };

  project = lake2nix.mkPackage {
    name = "Samples";
    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions [
        ./lakefile.toml
        ./lake-manifest.json
        ./lean-toolchain
        ./.helix
        ./Samples
      ];
    };
    postUnpack = ''
      rm "$sourceRoot/Samples"
      cp -r ${../samples} "$sourceRoot/Samples"
    '';
  };

  record-scene = pkgs.writers.writeNuBin "record-scene" {
    makeWrapperArgs = [
      "--prefix"
      "PATH"
      ":"
      (lib.makeBinPath [
        lean-tui
        pkgs.lean.lean-all
        pkgs.helix
        pkgs.tmux
        pkgs.vhs
      ])
      "--set"
      "FONTCONFIG_FILE"
      (pkgs.makeFontsConf { fontDirectories = [ pkgs.jetbrains-mono ]; })
      "--set"
      "TZDIR"
      "${pkgs.tzdata}/share/zoneinfo"
      "--set"
      "LC_ALL"
      "C.UTF-8"
      "--set"
      "SHELL"
      (lib.getExe pkgs.bashInteractive)
      "--set"
      "VHS_NO_SANDBOX"
      "true"
    ];
  } ./record-scene.nu;

  sharedTapes = [
    "common.tape"
    "open.tape"
  ];

  sceneNames = lib.pipe (builtins.readDir ./.) [
    builtins.attrNames
    (builtins.filter (file: lib.hasSuffix ".tape" file && !(builtins.elem file sharedTapes)))
    (map (lib.removeSuffix ".tape"))
  ];

  recordScene =
    scene:
    pkgs.stdenvNoCC.mkDerivation {
      name = "lean-tui-recording-${scene}";
      src = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions (
          map (tape: ./. + "/${tape}") (sharedTapes ++ [ "${scene}.tape" ]) ++ [ ./tmux.conf ]
        );
      };
      nativeBuildInputs = [ record-scene ];
      buildPhase = "HOME=$TMPDIR record-scene ${project} ${scene} --out $out";
      dontInstall = true;
    };

  scenes = lib.genAttrs sceneNames recordScene;
in
{
  inherit project scenes record-scene;
  all = pkgs.symlinkJoin {
    name = "lean-tui-recordings";
    paths = builtins.attrValues scenes;
  };
}
