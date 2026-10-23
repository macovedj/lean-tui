# Lean-TUI

A standalone terminal view that draws the proofs and programs you write in [Lean](https://lean-lang.org/) as interactive graphs, next to any editor: Helix, Neovim, Zed, Emacs or VS Code.

![Helix with lean-tui following the cursor](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/workflow.webp)

## Views

The view follows your cursor and picks a display mode for the code under it. See [display modes](https://codeberg.org/wvhulle/lean-tui/src/branch/main/docs/display-modes.md) for the details.

| Compact | Semantic tableau |
| --- | --- |
| ![Compact view](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/compact.webp) | ![Semantic tableau view](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/semantic_tableau.webp) |
| Hypotheses and goals before and after each tactic. | Every tactic step as a node in a proof tree. |
| **Compact tree** | **Data flow** |
| ![Compact tree view](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/compact_tree.webp) | ![Data flow view](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/data_flow.webp) |
| The proof tree on its side with the active state next to it. | Term-style proofs and functional programs, split into sub-states. |
| **Effect flow** | |
| ![Effect flow view](https://codeberg.org/wvhulle/lean-tui/media/branch/main/screenshots/effect_flow.webp) | |
| Return values and effects of monadic code. | |

## Quick start

1. Install `lean-tui` and the Lean-Prism language server (other ways in [setup](https://codeberg.org/wvhulle/lean-tui/src/branch/main/docs/setup.md)):

   ```bash
   nix profile install git+https://codeberg.org/wvhulle/lean-tui git+https://codeberg.org/wvhulle/lean-prism
   ```

2. Make `lean-prism serve` your editor's Lean language server. For Helix, in `languages.toml`:

   ```toml
   [language-server.lean-prism]
   command = "lean-prism"
   args = ["serve"]

   [[language]]
   name = "lean"
   language-servers = ["lean-prism"]
   ```

3. Open a Lean file in your editor, run `lean-tui` in a second terminal, and press `space k` on a proof in Helix. Lean-TUI tells you what it is waiting for until the first graph arrives.

## Documentation

- [Setup](https://codeberg.org/wvhulle/lean-tui/src/branch/main/docs/setup.md): installation, editor configuration, when the view updates, troubleshooting.
- [Usage](https://codeberg.org/wvhulle/lean-tui/src/branch/main/docs/usage.md): follow mode, panes, navigation and colors.
- [Display modes](https://codeberg.org/wvhulle/lean-tui/src/branch/main/docs/display-modes.md): what each view shows, with examples.
- [Keybindings](https://codeberg.org/wvhulle/lean-tui/src/branch/main/docs/keybindings.md): every key, generated from the source.

## Contributing

Issues and pull requests are welcome. Much of the work happens in the Lean back-end, [Lean-Prism](https://codeberg.org/wvhulle/lean-prism), so Lean users who don't write Rust can contribute there. See [CONTRIBUTING.md](https://codeberg.org/wvhulle/lean-tui/src/branch/main/CONTRIBUTING.md).
