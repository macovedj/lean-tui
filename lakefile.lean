import Lake
open Lake DSL

package «lean-tui-test» where
  version := v!"0.1.0"

require LeanPrism from git
  -- "https://codeberg.org/wvhulle/lean-prism.git" @ "main"
  "../lean-prism" @ "main" -- for local dev


require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git"

lean_lib Samples
