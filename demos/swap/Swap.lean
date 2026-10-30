theorem andSwap (p q : Prop) : p ∧ q ↔ q ∧ p := by
  apply Iff.intro

  intro h1
  rw [and_comm] at h1

  exact h1
  intro h2
  -- Wrong
  exact h2
  -- Correct: rw [and_comm] at h2; exact h2
