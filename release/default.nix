{
  pkgs,
  crane,
  lean-tui,
}:
let
  inherit (pkgs) lib;

  nightlyWithTargets =
    targets: pkgs.rust-bin.selectLatestNightlyWith (toolchain: toolchain.minimal.override { inherit targets; });

  mkMuslPackage =
    arch:
    let
      target = "${arch}-unknown-linux-musl";
      muslPkgs =
        if arch == "x86_64" then pkgs.pkgsCross.musl64 else pkgs.pkgsCross.aarch64-multiplatform-musl;
      muslCrane = (crane.mkLib pkgs).overrideToolchain (_: nightlyWithTargets [ target ]);
      cc = "${muslPkgs.stdenv.cc}/bin/${muslPkgs.stdenv.cc.targetPrefix}cc";
      ar = "${muslPkgs.stdenv.cc.bintools}/bin/${muslPkgs.stdenv.cc.targetPrefix}ar";
      targetEnv = builtins.replaceStrings [ "-" ] [ "_" ] target;
      muslArgs = lean-tui.cargoArgs // {
        pname = "lean-tui-static";
        doCheck = false;
        CARGO_BUILD_TARGET = target;
        "CARGO_TARGET_${lib.toUpper targetEnv}_LINKER" = cc;
        "CC_${targetEnv}" = cc;
        "AR_${targetEnv}" = ar;
      };
    in
    muslCrane.buildPackage (muslArgs // { cargoArtifacts = muslCrane.buildDepsOnly muslArgs; });

  muslPackages = lib.genAttrs [ "x86_64" "aarch64" ] mkMuslPackage;

  writeNuTool =
    name: tools: script:
    pkgs.writers.writeNuBin name {
      makeWrapperArgs = [
        "--prefix"
        "PATH"
        ":"
        (lib.makeBinPath tools)
      ];
    } script;

  releasePlzOnCodeberg =
    subcommand:
    writeNuTool subcommand [
      (nightlyWithTargets [ ])
      pkgs.release-plz
      pkgs.gitMinimal
    ] "^release-plz ${subcommand} --forge gitea --git-token $env.CODEBERG_TOKEN";
in
{
  inherit muslPackages;

  dist = pkgs.runCommand "lean-tui-dist" { } (
    lib.concatLines (
      [ "mkdir -p $out" ]
      ++ lib.mapAttrsToList (
        arch: package:
        "tar -czf $out/lean-tui-${arch}-unknown-linux-musl.tar.gz -C ${package}/bin lean-tui"
      ) muslPackages
    )
  );

  release = releasePlzOnCodeberg "release";
  release-pr = releasePlzOnCodeberg "release-pr";
  cached-build = writeNuTool "cached-build" [ pkgs.cachix ] ./cached-build.nu;
  check-commits = writeNuTool "check-commits" [
    pkgs.convco
    pkgs.gitMinimal
  ] "def --wrapped main [...range: string] { ^convco check ...$range }";
  upload-release-assets = writeNuTool "upload-release-assets" [
    pkgs.tea
    pkgs.coreutils
  ] ./upload-release-assets.nu;
}
