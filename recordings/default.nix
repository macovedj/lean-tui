{
  pkgs,
  lean4-nix,
  xtaskTool,
  lean-tui,
  lean-prism-server,
}:
let
  inherit (pkgs) lib;
  leanPkgs = pkgs.extend (lean4-nix.readToolchainFile ./lean-toolchain);
  lake2nix = leanPkgs.callPackage lean4-nix.lake { };

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
  };

  record-scene = xtaskTool {
    name = "record-scene";
    runtimeInputs = [
      lean-tui
      lean-prism-server
      leanPkgs.lean.lean-all
      pkgs.helix
      pkgs.libwebp
      pkgs.tmux
      pkgs.vhs
    ];
    wrapperArgs = [
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
  };

  sharedTapes = [
    "common.tape"
    "open.tape"
  ];

  sceneNames = lib.pipe (builtins.readDir ./tapes) [
    builtins.attrNames
    (builtins.filter (file: lib.hasSuffix ".tape" file && !(builtins.elem file sharedTapes)))
    (map (lib.removeSuffix ".tape"))
  ];

  recordScene =
    scene:
    pkgs.stdenvNoCC.mkDerivation {
      name = "lean-tui-recording-${scene}";
      src = lib.fileset.toSource {
        root = ./tapes;
        fileset = lib.fileset.unions (
          map (tape: ./tapes + "/${tape}") (sharedTapes ++ [ "${scene}.tape" ]) ++ [ ./tapes/tmux.conf ]
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
  publish-screenshots = xtaskTool {
    name = "publish-screenshots";
    runtimeInputs = [ pkgs.git ];
  };
}
