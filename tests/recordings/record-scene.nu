#!/usr/bin/env nu

def main [lean_project: path, scene: string, --out: path = "."]: nothing -> nothing {
  ^cp --recursive --no-preserve=mode $"($lean_project)/." .
  ^vhs $"($scene).tape"
  ^install -D $"($scene).gif" ($out | path join $"($scene).gif")
}
