{ pkgs, xtaskTool }:
{
  cached-build = xtaskTool {
    name = "cached-build";
    runtimeInputs = [ pkgs.cachix ];
  };
  check-commits = xtaskTool {
    name = "check-commits";
    runtimeInputs = [
      pkgs.convco
      pkgs.gitMinimal
    ];
  };
}
