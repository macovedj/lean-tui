import Lake
open Lake DSL

package «lean-tui-test» where
  version := v!"0.1.0"

require LeanPrism from "../../../lean-prism"

require mathlib from git
  "https://github.com/leanprover-community/mathlib4.git"

lean_lib Demos where
  roots := #[`prime.Prime, `swap.Swap, `sorting.Sorting, `monadic.Monadic]
