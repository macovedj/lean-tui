# Lean-TUI

Standalone interactive view for detailed visualization of programs and proofs that are written in [Lean](https://lean-lang.org/).

Watch a demo of some of the features: ![YouTube](https://youtu.be/whekHs2P1Ew).

(See the end of this page for a walkthrough.)

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

Add [LeanPrism](https://codeberg.org/wvhulle/lean-prism) as a Lake dependency.

In your `lakefile.toml`:

```toml
[[require]]
name = "LeanPrism"
git = "https://codeberg.org/wvhulle/lean-prism.git"
rev = "main"
```

Fetch the source code of the latest version (Lean only hosts on Git, does not provide prebuilt dependencies):

```bash
lake update LeanPrism
```

## Configuration

You need a working code editor with an LSP client.

### Helix (recommended)

You should not need to do any setup since Helix already has Lean configured out of the box.

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

Install the Lean4 extension from [VS Code Marketplace](https://marketplace.visualstudio.com/items?itemName=leanprover.lean4). (Not thoroughly tested, but should work)

### Zed

Either install the [Lean4 extension](https://zed.dev/extensions/lean4) or configure `lake serve` as LSP server.

## Quickstart

### 1. Split Your Terminal

You can choose between:

- Using multiple terminals (emulator windows) side-by-side (a terminal app is typically provided on Windows/Linux/Mac)
- Using a single terminal window with a terminal multiplexer (you'll need to install `zellij` or `tmux` with your system package manager)

### 2. Open your editor

Open your Lean file in your favorite (modal) editor that has a built-in LSP client (I recommend Helix, but Neovim, Zed, Kate also seem to have one). Lean will start up an LSP client hooked up to the default Lean LSP server, which might take a while. Normally, any diagnostics should appear in your editor as the LSP client finishes startup.

If you encounter problems, it is usually because of stale build artifacts or incorrect `lakefile.toml`.

### 3. Add Back-end Import

Add this to the top of your Lean file (or build target):

```lean
import LeanPrism
```

If you encounter issues, restart your LSP client using your editor's command. You should see a diagnostic saying that Lake is building your dependencies (including LeanPrism), which can take a while at the beginning.

### 4. Open `lean-tui`

Make sure you have followed the installation steps and added `~/.cargo/bin` to your path. Focus the other terminal (in any directory) and run this TUI (stands for terminal user interface):

```bash
lean-tui
```

This will launch the TUI in the second terminal, listening on a TCP port. Now you can switch back to your editor:

1. Move your cursor somewhere in a proof or function
2. Type in a proof or hover over tactics (`space,k` in Helix)

You might need to wait a bit the first time, because Lean needs to analyze the proof. Subsequent updates should be faster. I am open for improvements to make this faster.

### Key Bindings and Color Legend

Coloring is according to role.

- The cursor location is highlighted with bright foreground (white on black background).
- Inactive proof states are dimmed gray.
- Assumptions, parameters, and theorem hypotheses are blue
- The expected output type, theorem statements and all intermediate proof goals are orange.
- Completed nodes and proof states with no remaining open goals are purple.
- Errors are red.

If you don't like the colors or you are confused, please open a pull request or issue with the colors that you want.

Key bindings:

| Key   | Action                          |
| ----- | ------------------------------- |
| `↑/↓` | Navigate hypotheses and goals   |
| `g`   | Go to where item was introduced |
| `y`   | Copy to clipboard (OSC 52)      |
| `[/]` | Switch display mode             |
| `i`   | Toggle input pane               |
| `o`   | Toggle output pane              |
| `?`   | Help menu                       |
| `q`   | Quit                            |
| `f`   | File selection                  |
| `esc` | Close error banner              |
| `F`   | Toggle follow mode              |
| `L`   | Color legend                    |

The TUI follows your cursor in the editor automatically. It will automatically open the right display style. You can disable this behavior with `F` or you can manually select an open analyzed file with the file picker menu `f`. See below for a longer overview of each style.

## Graphs of Proof

### Example Proof

Below you can see (or go to [Codeberg](https://codeberg.org/wvhulle/lean-tui)) a simple but incomplete Lean proof:

```lean
import LeanPrism -- important
import Mathlib.Data.Set.Basic

theorem commutativityOfIntersections
    (s t : Set Nat) : s ∩ t = t ∩ s := by
  ext x
  apply Iff.intro

  intro h1
  rw [Set.mem_inter_iff, and_comm] at h1

  exact h1
  intro h2
  -- Wrong
  rw [ and_comm] at h2
  -- Correct: rw [Set.mem_inter_iff, and_comm] at h2
  exact h2
```

I use the above code fragment in the screenshots below. Notice that there is a deliberate type error. See how the rewrite is not sufficient to solve the proof goal and fails.

The deliberate error is there to show how errors are displayed by this program. As errors are an important part of proof development, they are attached to every display style.

### Compact View

Shows the proof as a list of hypotheses and goals. By default, displays the current state with the previous state (before the current tactic) side by side. You can hide the previous state pane with `p` and show the next state pane with `n` (disabled by default).

Select a hypothesis or goal and press `g` to move your cursor in your editor to the location where it was introduced. Press `y` to copy it to your clipboard for pasting elsewhere. When an error is active around the cursor position, it is also shown.

Hypotheses are blue. Goals are orange. Errors are red. Hypotheses that are added are highlighted in green, removed ones in red.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/before_after.png)

For the **VS Code users**: Yes, the official info view endorsed by the Lean organization is written for VS Code. However, I don't like VS Code and vendor lock-in. We should all try to move away from any Microsoft-owned product as soon as possible. Why? Take a look at for example ["Embrace, extend, and extinguish"](https://en.wikipedia.org/wiki/Embrace,_extend,_and_extinguish) but also the recent news.

For the **Neovim users**, yes there existed an info view [Lean plugin for Neovim](https://github.com/Julian/lean.nvim) already, but not yet for the other terminal editors (Zed, Kakoune, Helix, ...) since the Lean plugin relies on Neovim specific features. It is not a generic one.

### Semantic Tableau View

Visualizes your active proof as a semantic tableau. This is based on how logicians traditionally render their deductions. Errors are often too long and would not fit in a node alone, so you can click on it to expand it in a popup at the top of the screen. Press `escape` to close the popup or select another node.

Again, press `g` to jump to the selected hypothesis or goal. Use the arrow keys or `hjkl` to jump with keyboard only.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/semantic_tableau.png)

For **[Paperproof](https://github.com/Paper-Proof/paperproof) users**: Paperproof was a big inspiration for this view. However, it is Web-based. I used their approach to move most of the logic into Lean and add an RPC method there.

### Compact Tree View

Shows the proof as a compact Unicode tree on the left. On the right, you can see active hypotheses and goals. The tree follows the cursor position in your editor.

The same key bindings and colors apply as other views.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/tactic_tree.png)

## Graphs of Functional Programs

There are display modes focused on everything that is not a tactic proof. These modes should be useful for functional or monadic programming. You can test them out with some test files in [`./Samples`](./Samples)

### Data Flow (Experimental)

Use this for term-style proofs (without `by` or tactics). You can also use it for normal pure functional programs. It shows a graph of the local bindings and expected types throughout the program

The decomposition is based on a few built-in rules. However, not all Lean control flow has an accompanying rule. Please be patient as I add more rules or contribute yourself them to [`lean-prism`](https://codeberg.org/wvhulle/lean-prism). Rules can also be provided in user code and are picked up by `LeanPrism` (not tested yet).

Open the legend with `L` to see what all symbols and colors mean.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/data_flow.png)

### Effect Flow (Experimental)

When working doing programming in a monad, you are doing some kind of effectful computations. Every monadic assignment in `Lean` written as `let <- COMPUTATION` can be composed in two parts:

- What the computation produces, shown with a forward arrow
- What state changes are given back components (the effect), shown with a backward arrow

This decomposition is not implemented yet for all monads. Please contribute if you have time. You just need to specify how to compute the forward and backward expressions and have to label edges. You can also provide rules in user code (not tested yet).

The purple edges in the diagram are definition expansions (will be hide-able in future) and the blue ones are the monadic binds. Displaying errors is not implemented yet.

Open the legend with `L` to see what all symbols mean.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/effect_flow.png)

## Development

Let me know if you tried it out and encountered any issues! PRs are also welcome. For those, who don't know Rust (understandable), there is a large portion of Lean code in the backend that also needs to be worked on. Take a look there if you want to contribute as a Lean user.

See [`./CONTRIBUTING.md`](/CONTRIBUTING.md) for more.
