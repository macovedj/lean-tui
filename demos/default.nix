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
    name = "Demos";
    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.unions (
        [
          ./lakefile.toml
          ./lake-manifest.json
          ./lean-toolchain
          ./.helix
        ]
        ++ map (demo: lib.fileset.fileFilter (file: file.hasExt "lean") (./. + "/${demo}")) demoNames
      );
    };
  };

  record-scene = xtaskTool {
    name = "record-scene";
    runtimeInputs = [
      lean-tui
      lean-prism-server
      leanPkgs.lean.lean-all
      pkgs.ffmpeg
      pkgs.helix
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

  tapesInDirectory =
    directory:
    lib.pipe (builtins.readDir directory) [
      (lib.filterAttrs (file: kind: kind == "regular" && lib.hasSuffix ".tape" file))
      builtins.attrNames
      (map (lib.removeSuffix ".tape"))
    ];

  demoNames = lib.pipe (builtins.readDir ./.) [
    (lib.filterAttrs (entry: kind: kind == "directory" && tapesInDirectory (./. + "/${entry}") != [ ]))
    builtins.attrNames
  ];

  recordScene =
    demo: scene:
    pkgs.stdenvNoCC.mkDerivation {
      name = "lean-tui-recording-${scene}";
      src = lib.fileset.toSource {
        root = ./.;
        fileset = lib.fileset.unions [
          ./common.tape
          ./open.tape
          ./tmux.conf
          (./. + "/${demo}/${scene}.tape")
        ];
      };
      nativeBuildInputs = [ record-scene ];
      buildPhase = "HOME=$TMPDIR record-scene ${project} ${demo}/${scene}.tape --out $out/${demo}";
      dontInstall = true;
    };

  scenes = lib.pipe demoNames [
    (map (
      demo:
      map (scene: lib.nameValuePair scene (recordScene demo scene)) (tapesInDirectory (./. + "/${demo}"))
    ))
    lib.flatten
    builtins.listToAttrs
  ];
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
