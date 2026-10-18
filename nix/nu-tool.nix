{ pkgs }:
name: tools: script:
pkgs.writers.writeNuBin name {
  makeWrapperArgs = [
    "--prefix"
    "PATH"
    ":"
    (pkgs.lib.makeBinPath tools)
  ];
} script
