# Usage

## Following the editor

Lean-TUI follows your editor's cursor (follow mode): it switches to the file, definition and display mode under the cursor. Press `F` to turn follow mode off and keep the current view, and `f` to pick another open file by hand.

The status bar shows the cursor position and the LSP request that produced the current graph, for example `10:3 (hover) [follow]`.

The full list of keys is in [keybindings.md](keybindings.md); press `?` in lean-tui for the bindings of the current view.

## Panes

- The **input pane** (`i`) holds what goes in: hypotheses for proofs, parameters for functions.
- The **output pane** (`o`) holds what comes out: the goal for proofs, the expected type for functions.

## Selecting items

Click on hypotheses, goals and nodes, or move with the arrow keys or `hjkl`. On a selected item:

- `g` or `Enter` jumps the editor to where the item was introduced.
- `y` copies it to the clipboard.

Errors are attached to the node where they happen. Errors are often too long for a node, so click one to open it in a popup; `Esc` closes it.

`m` cycles the timing metrics shown per step (off, heartbeats, clock time). Elapsed time per tactic needs Lean-Prism's patched Lean build, see [Lean-Prism's supported versions](https://codeberg.org/wvhulle/lean-prism#supported-lean-versions).

## Colors

`L` opens a legend with the colors below.

### Pane borders

- Assumptions, parameters and theorem hypotheses: blue.
- Theorem statements and output types: orange.
- Inactive panes: gray.

### Node states

- The node under the cursor is emphasized (brighter or darker, depending on your terminal theme).
- Nodes with unsolved goals: yellow.
- Completed nodes and proof states without open goals: purple.
- Inactive proof states: dimmed gray.
- Nodes containing an error: red.

### Labels

| Tactic proofs | Term proofs   | Color  |
| ------------- | ------------- | ------ |
| Hypothesis    | Variable      | Blue   |
| Goal          | Expected type | Orange |

### Type diffs

Hypotheses added by a step are green, removed ones red. Goals that the next step closes are red.
