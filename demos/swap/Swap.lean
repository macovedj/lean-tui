-- Adapted from https://lean-lang.org/theorem_proving_in_lean4/Propositions-and-Proofs/ (`and_swap`):
-- rewritten with tactics, and the second direction deliberately misses the rewrite.
theorem and_swap (p q : Prop) : p ∧ q ↔ q ∧ p := by
  apply Iff.intro

  intro h1
  rw [and_comm] at h1

  exact h1
  intro h2
  exact h2
