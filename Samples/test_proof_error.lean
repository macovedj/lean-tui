import LeanPrism

theorem test : 1 + 1 = 3 := by
  rfl  -- Should fail because 1+1=2, not 3
