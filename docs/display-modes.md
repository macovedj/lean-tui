# Display modes

Different Lean code calls for different pictures. Lean-TUI picks a mode automatically from the code under the cursor: one of the proof views for tactic proofs, data flow for term-style code, effect flow for monadic code. Cycle through the modes with `[` and `]`.

The samples below live in [`demos/`](../demos/), one folder per sample, and each one names the official Lean documentation it is taken from.

## Compact

The proof state as a list of hypotheses and goals. By default the state before the current tactic is shown next to the current one.

This proof, adapted from `and_swap` in [Theorem Proving in Lean 4](https://lean-lang.org/theorem_proving_in_lean4/Propositions-and-Proofs/), lives in [`Swap.lean`](../demos/swap/Swap.lean) and has a deliberate error: `h2` is used without the rewrite, so it does not match the goal. Errors are part of proof development, so every mode shows them.

```lean
theorem and_swap (p q : Prop) : p ∧ q ↔ q ∧ p := by
  apply Iff.intro

  intro h1
  rw [and_comm] at h1

  exact h1
  intro h2
  exact h2
```

![Compact view](https://codeberg.org/wvhulle/lean-tui/media/branch/recordings/swap/compact.webp)

Unchanged items use the default color. Items added by the step are green; goals that get closed and hypotheses that get removed are red. `p` and `n` toggle the previous and next step panes.

## Semantic tableau

Each proof step is a node in a tree that follows the cursor in your editor. The recordings of this and the next mode use the prime number example from the [Lean homepage](https://lean-lang.org/), in [`Prime.lean`](../demos/prime/Prime.lean). Click on the goals and hypotheses of a node, or move between them with `hjkl`; `t` flips the tree orientation.

![Semantic tableau view](https://codeberg.org/wvhulle/lean-tui/media/branch/recordings/prime/semantic_tableau.webp)

This is close to how logicians draw [natural deduction](https://en.wikipedia.org/wiki/Natural_deduction) proofs. The layout is inspired by [Paperproof](https://github.com/Paper-Proof/paperproof).

## Compact tree

A compact semantic tableau: the proof tree on its side on the left, the hypotheses and goals active at the selected step on the right. `jk` selects items, `hl` scrolls through the steps.

![Compact tree view](https://codeberg.org/wvhulle/lean-tui/media/branch/recordings/prime/compact_tree.webp)

## Data flow

Not every proof uses tactics. Term-style proofs work forwards and look like ordinary functional programs. This mode splits such code into related sub-states, built from both the concrete and the abstract syntax tree. Each node holds the expected type at that point and the variables in scope.

Merge sort from [Functional Programming in Lean](https://lean-lang.org/functional_programming_in_lean/Programming___-Proving___-and-Performance/More-Inequalities/), in [`Sorting.lean`](../demos/sorting/Sorting.lean):

```lean
def merge [Ord α] (xs : List α) (ys : List α) : List α :=
  match xs, ys with
  | [], _ => ys
  | _, [] => xs
  | x'::xs', y'::ys' =>
    match Ord.compare x' y' with
    | .lt | .eq => x' :: merge xs' (y' :: ys')
    | .gt => y' :: merge (x'::xs') ys'
```

![Data flow view](https://codeberg.org/wvhulle/lean-tui/media/branch/recordings/sorting/data_flow.webp)

The decomposition follows a set of built-in rules, and not every Lean control-flow construct has one yet.

## Effect flow

In monadic code every `let x ← computation` combines:

- a **return value**: what the computation returns, drawn with a forward arrow;
- an **effect**: how it changes the context, such as state writes, drawn as an icon under the return value. Effects of nested monadic calls bubble up into a list.

The letter counter from [Functional Programming in Lean](https://lean-lang.org/functional_programming_in_lean/Monad-Transformers/A-Monad-Construction-Kit/#state-monad), in [`Monadic.lean`](../demos/monadic/Monadic.lean):

```lean
    | c :: cs =>
      let st ← get
      let st' ←
        if c.isAlpha then
          if vowels.contains c then
            pure {st with vowels := st.vowels + 1}
          else if consonants.contains c then
            pure {st with consonants := st.consonants + 1}
          else -- modified or non-English letter
            pure st
        else throw (.notALetter c)
      set st'
      loop cs
```

![Effect flow view](https://codeberg.org/wvhulle/lean-tui/media/branch/recordings/monadic/effect_flow.webp)

Nested calls and pure steps are hidden by default; `e` expands them.
