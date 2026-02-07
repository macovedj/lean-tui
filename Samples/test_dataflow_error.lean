import LeanPrism

def testFunc (x : Nat) : Nat :=
  let y := x + 1
  let z := y + "hello"  -- Type error here
  z
