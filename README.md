# Lean-TUI

**Standalone interactive view that shows a graph view of proofs that are written in [Lean](https://lean-lang.org/).**

Shows proof structure, hypotheses, and goals (comparable to the VS Code Web-based info view but in your terminal). However, this info view can also show the **complete graph structure** of the complete proof, not only the current open goals. It also tracks your edits in editor and reversely you can use the info view to jump to locations in the editor.

Type errors are shown as close as possible to the relevant location.

## Installation

### 1. Install `lean-tui` from Crates.io

Install Rust (through [`rustup`](https://rustup.rs/)) if you haven't compiled Rust programs before. Then install this crate as a binary in your user with:

```bash
cargo install lean-tui
```

If `~/.cargo/bin` is in your path, you can now run this program with `lean-tui`.

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

Configure your editor to use `lake serve` as the Lean LSP server. This connects to lean-prism which provides the extended RPC methods.

For example, for Helix:

```toml
# .helix/languages.toml
[language-server.lean]
command = "lake"
args = ["serve"]

[[language]]
language-servers = ["lean"]
name = "lean"
```

Add this to the top of your Lean file (or build target):

```lean
import LeanPrism
```

If you encounter issues, restart your LSP client using your editor's command. You should see a diagnostic saying that Lake is building your dependencies (including LeanPrism), which can take a while at the beginning.

## Quickstart

### 1. Split Your Terminal

You can choose between:

- Using multiple terminals (emulator windows) side-by-side (a terminal app is typically provided on Windows/Linux/Mac)
- Using a single terminal window with a terminal multiplexer (you'll need to install `zellij` or `tmux` with your system package manager)

Open your Lean file in your favorite (modal) editor that has a built-in LSP client (I recommend Helix, but Neovim, Zed, Kate also seem to have one).

Split terminal. Launch the TUI in same directory in the second terminal with `lean-tui`.

### 2. Start Writing Proofs

Switch back to your editor:

1. Move your cursor somewhere in a proof or function
2. Type in a proof or hover over tactics (`space,k` in Helix)

You might need to wait a bit the first time, because Lean needs to analyze the proof. Subsequent updates should be faster. I am open for improvements to make this faster.

Coloring is according to role. Assumptions are blue and goals are orange. Errors are red. Completed nodes are purple.

Switch to the TUI. Key bindings:

| Key   | Action                          |
| ----- | ------------------------------- |
| `↑/↓` | Navigate hypotheses and goals   |
| `g`   | Go to where item was introduced |
| `y`   | Copy to clipboard (OSC 52)      |
| `[/]` | Switch display mode             |
| `?`   | Help menu                       |
| `q`   | Quit                            |
| `f`   | File selection                  |
| `esc` | Close error banner              |
| `F`   | Toggle follow mode              |
| `L`   | Color legend                    |

The TUI follows your cursor in the editor automatically. It will automatically open the right display style. You can disable this behaviour with `F` or you can manually select an open analyzed file with the file picker menu `f`. See below for a longer overview of each style.

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

I use the above code fragment in the screenshots below. Notice that there is a deliberate type error. This is to show how errors are displayed. As errors are an important part of proof development, they are attached to every display style.

### Plain view

Just shows the proof as a list of hypotheses and goals. When an error is active around the cursor position, it also shown. Select a hypotheses or goal and press `g` to move your cursor in your editor to the location where it was introduced. Press `y` to copy it to you clipboard for pasting elsewhere.

Hypotheses are blue. Goals are orange. Errors are red.

https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/plain_view.png

![(Screenshot at Codeberg)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/plain_view.png)

For the **VS Code users**: Yes, the official info view endorsed by the Lean organisation is written for VS Code, However, I don't like VS Code and vendor lock-in. We should all try to move away from any Microsoft-owned product as soon as possible. Why? Have a look at for example ["Embrace, extend, and extinguish"](https://en.wikipedia.org/wiki/Embrace,_extend,_and_extinguish) but also the recent news.

For the **Neovim users**, yes there existed an info view [Lean plugin for Neovim](https://github.com/Julian/lean.nvim) already, but not yet for the other terminal editors (Zed, Kakoune, Helix, ...) since the Lean plugin relies on Neovim specific features. It is not a generic one.

### Semantic tableau view

Visualizes your active proof as a semantic tableau. This is based on how logicians traditionaly render their deductions. Errors are often too long and would not fit in a node alone, so you can click on it to expand it in a popup at the top of the screen. Press `escape` to close the popup or select another node.

Again, press `g` to jump to the selected hypothesis or goal. Use the arrow keys or `hjkl` to jump with keyboard only.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/semantic_tableau.png) |

For **[Paperproof](https://github.com/Paper-Proof/paperproof) users**: Paperproof was a big inspiration for this view. However, it is Web-based. I used their approach to move most of the logic into Lean and add an RPC method there.

### Before/after view

Shows active goal state and on the left also the previous state. Assumptions or hypotheses that are added are highlighed in green. Removed ones are highlighed in red. Errors from cursor position are also shown if present.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/before_after.png)

### Tactic tree view

Shows a rendering of the proof displayed as compact unicode tree on the left. On the right, you can see, active hypotheses and goals. The tree will update its location by focusing the active proof state at the cursor position.

The same keybindings and colors apply as other views.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/tactic_tree.png)

## Graphs of Functional Programs

There are display modes focused on everything that is not a tactic proof. These modes should be useful for functional or monadic programming. You can test them out with some test files in [`./Samples`](./Samples)

### Data Flow (experimental)

Use this for term-style proofs (without `by` or tactics). You can also use it for normal pure functional programs. It shows a graph of the local bindings and expected types throughout the program

The decomposition is based on a few built-in rules. However, not all Lean control flow has an accompanying rule. Please be patient as I add more rules or contribute them to [`lean-prism`](https://codeberg.org/wvhulle/lean-prism).

Open the legend with `L` to see what all symbols and colors mean.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/data_flow.png)

### Effect flow (experimental)

When working doing programming in a monad, you are doing some kind of effectful computations. Every monadic assignment in `Lean` written as `let <- COMPUTATION` can be composed in two parts:

- What the computation produces
- What state changes are given back components (the effect)

The purple edges in the diagram are definition expansions (will be hideable in future) and the blue ones are the monadic binds. Displaying errors is not implemented yet.

Open the legend with `L` to see what all symbols mean.

![(Screenshot in repo)](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/effect_flow.png)

## Development

Let me know if you tried it out and encountered any issues! PRs are also welcome.

See [`./CONTRIBUTING.md`](/CONTRIBUTING.md) for more.
