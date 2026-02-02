import LeanDag
import Batteries.Data.Nat.Basic
-- Sample file demonstrating monadic programming with do blocks in Lean 4

def divide (x y : Nat) : Option Nat :=
  if y == 0 then none else some (x / y)

def safeSqrt (n : Nat) : Option Nat :=
  if n == 0 then some 0
  else
    let guess := n / 2
    if guess * guess <= n && (guess + 1) * (guess + 1) > n
    then some guess
    else some (Nat.sqrt n)

def computation (a b c : Nat) : Option Nat := do
  let quotient ← divide a b
  let root ← safeSqrt quotient
  let final ← divide (root + c) 2

  pure (final * 3)

def parseDigit (c : Char) : Option Nat :=
  if '0' ≤ c && c ≤ '9'
  then some (c.toNat - '0'.toNat)
  else none

def parseNumber (s : String) : Option Nat := do
  guard (s.length > 0)
  let digits ← s.toList.mapM parseDigit
  pure (digits.foldl (fun acc d => acc * 10 + d) 0)

def validateAndCompute (input : String) : Option String := do
  let n ← parseNumber input
  guard (n > 0)
  let result ← computation n 3 5
  -- pure s!"Result: {result}"

inductive Expr where
  | num : Int → Expr
  | add : Expr → Expr → Expr
  | mul : Expr → Expr → Expr
  | div : Expr → Expr → Expr
  deriving Repr

def evalExpr : Expr → Option Int
  | Expr.num n => some n
  | Expr.add e1 e2 => do
      let v1 ← evalExpr e1
      let v2 ← evalExpr e2
      pure (v1 + v2)
  | Expr.mul e1 e2 => do
      let v1 ← evalExpr e1
      let v2 ← evalExpr e2
      pure (v1 * v2)
  | Expr.div e1 e2 => do
      let v1 ← evalExpr e1
      let v2 ← evalExpr e2
      guard (v2 ≠ 0)
      pure (v1 / v2)

def exampleExpr : Expr :=
  Expr.div (Expr.add (Expr.num 10) (Expr.mul (Expr.num 2) (Expr.num 3))) (Expr.num 4)

#eval evalExpr exampleExpr

#eval validateAndCompute "42"
#eval validateAndCompute "0"
#eval validateAndCompute "abc"

def main : IO Unit := do
  IO.println "Testing monadic computations..."

  match computation 100 5 7 with
  | some r => IO.println s!"computation 100 5 7 = {r}"
  | none => IO.println "computation failed"

  match evalExpr exampleExpr with
  | some v => IO.println s!"Expression evaluates to: {v}"
  | none => IO.println "Expression evaluation failed (division by zero)"

  for input in ["123", "456", "0", "invalid"] do
    match validateAndCompute input with
    | some msg => IO.println msg
    | none => IO.println s!"Failed to process: {input}"
