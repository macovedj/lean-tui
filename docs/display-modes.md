# Display modes

Different Lean code calls for different pictures. Lean-TUI picks a mode automatically from the code under the cursor: one of the proof views for tactic proofs, data flow for term-style code, effect flow for monadic code. Cycle through the modes with `[` and `]`.

The samples below live in [`demos/`](../demos/).

## Compact

The proof state as a list of hypotheses and goals. By default the state before the current tactic is shown next to the current one.

This proof from [`Swap.lean`](../demos/swap/Swap.lean) has a deliberate error: `h2` is used without the rewrite, so it does not match the goal. Errors are part of proof development, so every mode shows them.

```lean
theorem andSwap (p q : Prop) : p ∧ q ↔ q ∧ p := by
  apply Iff.intro

  intro h1
  rw [and_comm] at h1

  exact h1
  intro h2
  -- Wrong
  exact h2
  -- Correct: rw [and_comm] at h2; exact h2
```

![Compact view](../demos/swap/compact.webp)

Unchanged items use the default color. Items added by the step are green; goals that get closed and hypotheses that get removed are red. `p` and `n` toggle the previous and next step panes.

## Semantic tableau

Each proof step is a node in a tree that follows the cursor in your editor. Click on the goals and hypotheses of a node, or move between them with `hjkl`; `t` flips the tree orientation.

![Semantic tableau view](../demos/prime/semantic_tableau.webp)

This is close to how logicians draw [natural deduction](https://en.wikipedia.org/wiki/Natural_deduction) proofs. The layout is inspired by [Paperproof](https://github.com/Paper-Proof/paperproof).

## Compact tree

A compact semantic tableau: the proof tree on its side on the left, the hypotheses and goals active at the selected step on the right. `jk` selects items, `hl` scrolls through the steps.

![Compact tree view](../demos/prime/compact_tree.webp)

## Data flow

Not every proof uses tactics. Term-style proofs work forwards and look like ordinary functional programs. This mode splits such code into related sub-states, built from both the concrete and the abstract syntax tree. Each node holds the expected type at that point and the variables in scope.

From [`Sorting.lean`](../demos/sorting/Sorting.lean):

```lean
def insert (x : Nat) : List Nat → List Nat
  | [] => [x]
  | y :: ys => if x ≤ y then x :: y :: ys else  y :: insert x ys
```

![Data flow view](../demos/sorting/data_flow.webp)

The decomposition follows a set of built-in rules, and not every Lean control-flow construct has one yet.

## Effect flow

In monadic code every `let x ← computation` combines:

- a **return value**: what the computation returns, drawn with a forward arrow;
- an **effect**: how it changes the context, such as state writes, drawn as an icon under the return value. Effects of nested monadic calls bubble up into a list.

From [`Monadic.lean`](../demos/monadic/Monadic.lean):

```lean
def counterSequence : StateM Counter Nat := do
  let v1 ← increment
  let v2 ← double
  let v3 ← increment
  pure (v1 + v2 + v3)
```

![Effect flow view](../demos/monadic/effect_flow.webp)

Nested calls and pure steps are hidden by default; `e` expands them.
