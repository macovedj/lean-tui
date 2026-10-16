#!/usr/bin/env nu

def --wrapped main [...build_args: string]: nothing -> nothing {
  if ($env.CACHIX_AUTH_TOKEN? | is-empty) {
    ^nix build --print-build-logs ...$build_args
  } else {
    ^cachix watch-exec wvhulle -- nix build --print-build-logs ...$build_args
  }
}
