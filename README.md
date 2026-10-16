# Lean-TUI

Standalone interactive view for detailed visualization of programs and proofs that are written in [Lean](https://lean-lang.org/).

![Helix with lean-tui following the cursor](./screenshots/workflow.gif)

Watch a demo of some of the features: <https://youtu.be/whekHs2P1Ew>. See the bottom of this page for a usage guide.

<!--toc:start-->

- [Lean-TUI](#lean-tui)
  - [Installation](#installation)
    - [1. Install `lean-tui` from Crates.io](#1-install-lean-tui-from-cratesio)
    - [2. Add `LeanPrism` Dependency](#2-add-leanprism-dependency)
  - [Configuration](#configuration)
    - [Helix](#helix)
    - [Neovim](#neovim)
    - [VS Code](#vs-code)
    - [Zed](#zed)
  - [Quickstart](#quickstart)
    - [1. Split Your Terminal](#1-split-your-terminal)
    - [2. Open Your Editor](#2-open-your-editor)
    - [3. Add Back-End Import](#3-add-back-end-import)
    - [4. Open `lean-tui`](#4-open-lean-tui)
    - [5. Start Writing Lean](#5-start-writing-lean)
  - [User Guide](#user-guide)
    - [Keyboard](#keyboard)
    - [Pane borders](#pane-borders)
    - [Node State Colors](#node-state-colors)
    - [Label Colors](#label-colors)
    - [Diff Colors for Types](#diff-colors-for-types)
  - [Display Styles](#display-styles)
    - [Compact View](#compact-view)
    - [Semantic Tableau View](#semantic-tableau-view)
    - [Compact Tree View](#compact-tree-view)
    - [Data Flow](#data-flow)
    - [Effect Flow](#effect-flow)
  - [Development](#development)

<!--toc:end-->

## Installation

For those that just want to get started quickly

### 1. Install `lean-tui` from Crates.io

Install Rust (through [`rustup`](https://rustup.rs/)) if you haven't compiled Rust programs before. Then install this crate as a binary in your user with:

```bash
cargo install lean-tui
```

Or if you want the latest unstable version,

```bash
cargo install https://codeberg.org/wvhulle/lean-tui
```

Now, make sure that `~/.cargo/bin` is in your path to be able to run `lean-tui` (without the `~/.cargo/bin/lean-tui` prefix).

### 2. Add `LeanPrism` Dependency

Add [LeanPrism](https://codeberg.org/wvhulle/lean-prism) as a Lake dependency, pinned to the branch that matches the `lean-toolchain` of your project. For `leanprover/lean4:v4.33.0`, in your `lakefile.toml`:

```toml
[[require]]
name = "LeanPrism"
git = "https://codeberg.org/wvhulle/lean-prism.git"
rev = "lean-v4.33"
```

Alternatively, in `lakefile.lean`:

```lean
require LeanPrism from "https://codeberg.org/wvhulle/lean-prism.git" @ "lean-v4.33"
```

> [!WARNING]
> LeanPrism runs inside the Lean language server and uses unstable compiler internals, so a LeanPrism revision works with exactly one Lean minor version. Lake builds it with your project's toolchain, not its own. Pick the `rev` for your `lean-toolchain` from the [supported versions table](https://codeberg.org/wvhulle/lean-prism#installation) and change it whenever you upgrade Lean. A mismatch shows up as errors such as `object file '.../Std/Internal/Async.olean' does not exist`. Avoid `rev = "main"`: it follows whichever Lean version LeanPrism currently targets.

Optionally, pre-fetch the source of this library and build it:

```bash
lake update LeanPrism
lake build LeanPrism
```

## Configuration

You need a working code editor with an LSP client.

### Helix

This is the recommended editor and used by the author. You should not need to do any setup since Helix already has Lean configured out of the box.

If you are using an older version, configure Helix to use `lake serve` as the Lean LSP server like this

```toml
# .helix/languages.toml
[language-server.lean]
command = "lake"
args = ["serve"]

[[language]]
language-servers = ["lean"]
name = "lean"
```

### Neovim

Configure the command `lake serve` as a language server for `.lean` files. See [official Neovim LSP docs](https://neovim.io/doc/user/lsp.html).

### VS Code

Install the Lean extension from [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=leanprover.lean4). The VS Code info view will open automatically, but you might want to close it since this UI offers slightly more.

### Zed

You can choose:

- Configure `lake serve` as LSP server, see [official docs](https://zed.dev/docs/configuring-languages) (recommended)
- Install the [Lean extension](https://zed.dev/extensions/lean4)

## Quickstart

### 1. Split Your Terminal

You can choose between:

- Using multiple terminals (emulator windows) side-by-side (a terminal app is typically provided on Windows/Linux/Mac)
- Using a single terminal window with a terminal multiplexer (you'll need to install `zellij` or `tmux` with your system package manager)

### 2. Open Your Editor

Open your Lean file in your editor (with an LSP client). Lean will start up an LSP client hooked up to the default Lean LSP server, which might take awhile. Normally, any diagnostics should appear in your editor as the LSP client finishes start up.

If you encounter problems, it is usually because of stale build artifacts or incorrect `lakefile.toml`.

### 3. Add Back-End Import

Most of the logic behind this UI uses a back-end written in Lean. To import it,
add this to the top of your Lean file (or build target):

```lean
import LeanPrism
```

The Lake LSP server now needs to be restarted. You can do this as follows:

- VS Code: Click on the `\forall` symbol at the corner and select "restart server"
- Helix: type command `:lsp-restart`
- Neovim / others: restart your LSP client using your editor's command

You should now see a diagnostic (or notification) saying that Lake is building your `LeanPrism`, which can take awhile at the beginning.

### 4. Open `lean-tui`

Make sure you have followed the installation steps and added `~/.cargo/bin` to your path. Focus the other terminal (in any directory) and run this TUI (stands for terminal user interface):

```bash
lean-tui
```

This will launch the TUI in the second terminal, listening on a TCP port.

### 5. Start Writing Lean

Now you can switch back to your editor. Move your cursor somewhere in a proof or function using keyboard or mouse.

Depending on your editor, the Lean-TUI view will update after specific events:

- Helix, one of the following:
  - Open the completion menu in insert mode: `ctrl+x`
  - Open documentation popup in normal mode: `space,k`
  - Make edits in INSERT mode (should trigger the completions)
- Neovim: same as Helix, but you may configure more frequent updates using Lua
- VS Code:
  - Positioning cursor with mouse or keyboard
  - Opening completions
  - Making edits

The first rendering of a proof or function in a new project might be a bit slow, since the initialization of the TCP server happens lazily. Subsequent updates should be much faster.

## User Guide

### Keyboard

Normally, the view should follow your changes in the editor. Otherwise, you can also navigate with single-letter keyboard shortcuts:

| Key       | Action                        |
| --------- | ----------------------------- |
| `←/↑/↓/→` | Navigate hypotheses and goals |
| `h/j/k/l` | Navigate hypotheses and goals |
| `[/]`     | Switch display mode           |
| `?`       | Help menu                     |
| `q`       | Quit                          |
| `L`       | Color legend                  |

The TUI follows your cursor in the editor automatically (called "follow mode") and will automatically open the view with the right file and display style.

You can disable this behavior with `F` or you can manually select an open analyzed file with the file picker menu `f`.
Errors are often too long and would not fit in a node alone, so you can click on an error in to expand it in a popup at the top of the screen. Press `escape` to close the popup or select another node.

| Key   | Action             |
| ----- | ------------------ |
| `f`   | File selection     |
| `F`   | Toggle follow mode |
| `esc` | Dismiss error      |

All views have the following key bindings for common operations:

| Key | Action                          |
| --- | ------------------------------- |
| `g` | Go to where item was introduced |
| `y` | Copy to clipboard               |
| `i` | Toggle input pane               |
| `o` | Toggle output pane              |

The input pane's content depends on your program's style. For functional programs, it contains function parameters. For proofs, it contains the hypotheses. The output pane is the expected type or the proposition to be proven.

### Pane Borders

Panes are large sub-windows in the TUI. Their border is colored consistently between different display modes:

- Assumptions, parameters, and theorem hypotheses are blue
- Theorem statements, and output types have an orange border.
- Inactive panes are gray

### Node State Colors

The border colors of the nodes also change dynamically:

- The border the node under cursor will be emphasized depending on your terminal's color scheme (brighter or lighter).
- Nodes with unsolved goals have a yellow border.
- Completed nodes and proof states with no remaining open goals are purple.

Furthermore:

- Inactive proof states are dimmed gray.
- Nodes containing an error are red.

### Label Colors

For tactic based proofs:

| Label      | Color  |
| ---------- | ------ |
| Hypothesis | Blue   |
| Goal       | Orange |

In term-based proofs, the colors are analogous

| Label         | Color  |
| ------------- | ------ |
| Variable      | Blue   |
| Expected type | Orange |

### Diff Colors for Types

Hypotheses types that are added are highlighted in green, removed ones in red. Goal types that will be closed in the next step are red.

(Thanks to the expressive metaprogramming facilities of Lean, it easy to diff metavariable type values across state boundaries)

## Display Styles

Different types of Lean programs or proof may require different visualization types. The categories currently supported:

- Tactics graph
- Data flow graph
- Effect flow graph

Switching between them happens automatically when you change between different parts of your program. If the autodetection does not work properly, you can also use:

| Key   | Action              |
| ----- | ------------------- |
| `[/]` | Switch display mode |

### Compact View

Shows the proof as a list of hypotheses and goals. By default, displays the current state with the previous state (before the current tactic) side by side.

Below you can see (or go to [Codeberg](https://codeberg.org/wvhulle/lean-tui)) a simple but incomplete Lean proof from [`./tests/samples/swap_tactic.lean`](./tests/samples/swap_tactic.lean):

```lean
import LeanPrism

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

Notice that there is a deliberate type error. See how `h2` is used without the rewrite, so it does not match the goal. The deliberate error is there to show how errors are displayed by this program. As errors are an important part of proof development, they are attached to every display style.

Lean-TUI will visualize the proof as follows:

![Compact view](./screenshots/compact.gif)

By default the previous state is shown as well. Things that didn't change will have the default color (white or black). Things added in the current state are green. Goals that will be closed or hypotheses removed are red.

| Key | Action              |
| --- | ------------------- |
| `p` | Show previous state |
| `n` | Show next state     |
| `y` | Copy to clipboard   |

### Semantic Tableau View

Intuitively, your proof consists of steps and each step is a node in this tree diagram. The tree follows the cursor position in your editor.

![Semantic tableau view](./screenshots/semantic_tableau.gif)

You can click on the goals and hypotheses of each node. You can also jump between clickable items with your keyboard:

| Key       | Action                        |
| --------- | ----------------------------- |
| `←/↑/↓/→` | Navigate hypotheses and goals |
| `h/j/k/l` | Navigate hypotheses and goals |

This view is similar to how computer scientists and logicians talk about type theories, see [Natural Deduction](https://en.wikipedia.org/wiki/Natural_deduction). The implementation is based on **[Paperproof](https://github.com/Paper-Proof/paperproof).

### Compact Tree View

This is a more compact version of the semantic tableau with the tree on its side on the left.

On the right, you can see active hypotheses and goals, but only the ones currently active.

![Compact tree view](./screenshots/compact_tree.gif)

Not all proofs are written using tactics. Lean has support for both tactic-based and term-based proofs. Tactic-based proofs work backwards. Term-based proofs work forwards and are almost indistinguishable from normal functional programs.

### Data Flow

This view mode is for term-styled proofs and purely functional programming. It does a decomposition of your program into different related sub-states. The decomposition is built using a combination of CST and AST traversal. Each node contains the expected type at the focused part of the program and all active variables.

The following screenshot is a visualization of the [`./tests/samples/sorting.lean`](./tests/samples/sorting.lean) file:

```lean
def insert (x : Nat) : List Nat → List Nat
  | [] => [x]
  | y :: ys => if x ≤ y then x :: y :: ys else  y :: insert x ys
```

![Data flow view](./screenshots/data_flow.gif)

The decomposition is based on a few built-in rules. However, not all Lean control flow has an accompanying rule.

### Effect Flow

When working doing programming in a monad, you are doing some kind of effectful computations. Every monadic assignment in `Lean` written as `let <- COMPUTATION` can be viewed as a combination of:

- **Return value**: What the computation **returns** (if any), shown with a forward arrow.
- **Effect**: How the call-site is affected, such as state changes, shown with an icon below the return value. Effects may be hidden in nested monadic calls, these are bubbled up and shown in a list.

As example, here is a visualization of

```lean
def counterSequence : StateM Counter Nat := do
  let v1 ← increment
  let v2 ← double
  let v3 ← increment
  pure (v1 + v2 + v3)
```

![Effect flow view](./screenshots/effect_flow.gif)

If you want to see the leaf nodes, use the key binding `e` (expand). By default, nested calls and function definitions are hidden.

## Development

Let me know if you tried it out and encountered any issues! PRs are also welcome. For those, who don't know Rust (understandable), there is a large portion of Lean code in the backend that also needs to be worked on. Take a look there if you want to contribute as a Lean user.

See [`./CONTRIBUTING.md`](CONTRIBUTING.md) for more.
