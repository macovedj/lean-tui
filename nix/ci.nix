{ pkgs, writeNuTool }:
{
  cached-build = writeNuTool "cached-build" [ pkgs.cachix ] ./scripts/cached-build.nu;
  check-commits = writeNuTool "check-commits" [
    pkgs.convco
    pkgs.gitMinimal
  ] "def --wrapped main [...range: string] { ^convco check ...$range }";
}
