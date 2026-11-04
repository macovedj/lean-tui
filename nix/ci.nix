{ pkgs, nightlyWithTargets }:
let
  inherit (pkgs) lib;

  nuScriptTool =
    {
      script,
      name,
      runtimeInputs ? [ ],
      wrapperArgs ? [ ],
    }:
    pkgs.runCommand name
      {
        nativeBuildInputs = [ pkgs.makeBinaryWrapper ];
        meta.mainProgram = name;
      }
      ''
        makeWrapper ${lib.getExe pkgs.nushell} $out/bin/${name} \
          --add-flags ${script} \
          --add-flags ${name} \
          ${lib.optionalString (runtimeInputs != [ ]) "--prefix PATH : ${lib.makeBinPath runtimeInputs}"} \
          ${lib.escapeShellArgs wrapperArgs}
      '';

  nixBuiltPathsTool = toolArgs: nuScriptTool (toolArgs // { script = ../ci/nix-built-paths.nu; });
  releaseTaskTool = toolArgs: nuScriptTool (toolArgs // { script = ../ci/release-tasks.nu; });

  releasePlzInputs = [
    (nightlyWithTargets [ ])
    pkgs.release-plz
    pkgs.gitMinimal
  ];

  exportBuiltPathsHook = nixBuiltPathsTool { name = "export-built-paths-hook"; };

  exportingBuildTool =
    toolArgs:
    nixBuiltPathsTool (
      toolArgs
      // {
        wrapperArgs = [
          "--set"
          "EXPORT_HOOK_PROGRAM"
          (lib.getExe exportBuiltPathsHook)
        ];
      }
    );
in
{
  ciTaskTests = pkgs.runCommand "lean-tui-ci-task-tests" { nativeBuildInputs = [ pkgs.nushell ]; } ''
    nu ${../ci}/test-ci-tasks.nu
    touch $out
  '';

  ciTaskTools = {
    release = releaseTaskTool {
      name = "release";
      runtimeInputs = releasePlzInputs;
    };
    release-pr = releaseTaskTool {
      name = "release-pr";
      runtimeInputs = releasePlzInputs;
    };
    collect-release-archives = releaseTaskTool { name = "collect-release-archives"; };
    upload-release-assets = releaseTaskTool {
      name = "upload-release-assets";
      runtimeInputs = [ pkgs.tea ];
    };
    check-commits = releaseTaskTool {
      name = "check-commits";
      runtimeInputs = [
        pkgs.convco
        pkgs.gitMinimal
      ];
    };
    build-and-export-paths = exportingBuildTool { name = "build-and-export-paths"; };
    build-unless-cached = exportingBuildTool { name = "build-unless-cached"; };
    import-exported-paths = nixBuiltPathsTool { name = "import-exported-paths"; };
    upload-exported-paths = nixBuiltPathsTool {
      name = "upload-exported-paths";
      runtimeInputs = [ pkgs.cachix ];
    };
  };
}
