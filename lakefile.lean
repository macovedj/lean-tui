import Lake
open Lake DSL

package «lean-tui-test» where
  version := v!"0.1.0"

require LeanDag from git
  "https://codeberg.org/wvhulle/lean-dag.git" @ "main"
  -- "../lean-dag" @ "main" -- for local dev


require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git"

@[default_target]
lean_lib Samples
