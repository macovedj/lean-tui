# Setup

Lean-TUI is a front-end. The proof and program graphs come from [Lean-Prism](https://codeberg.org/wvhulle/lean-prism), which runs inside the Lean language server of your editor and streams the graph under your cursor to `lean-tui` over TCP.

## Install with Nix

Both projects are flakes. Add them as inputs of your NixOS, Home Manager or project flake:

```nix
{
  inputs = {
    lean-tui.url = "git+https://codeberg.org/wvhulle/lean-tui";
    lean-prism.url = "git+https://codeberg.org/wvhulle/lean-prism";
  };
}
```

and put the packages where you keep your tools, for example in a Home Manager module that receives `inputs` and `pkgs`:

```nix
home.packages = [
  inputs.lean-tui.packages.${pkgs.stdenv.hostPlatform.system}.default
  inputs.lean-prism.packages.${pkgs.stdenv.hostPlatform.system}.default
];
```

Use `environment.systemPackages` on NixOS, or a devshell's `packages` to keep them per project. The Lean-Prism flake builds for `x86_64-linux` only.

To try both without installing anything:

```bash
nix shell git+https://codeberg.org/wvhulle/lean-tui git+https://codeberg.org/wvhulle/lean-prism
```

The flakes use the binary cache `wvhulle.cachix.org`; accept its settings when Nix asks.

## Install `lean-tui` without Nix

With Cargo, the build needs Lean-Prism's protocol schema, named by `LEAN_DAG_SCHEMA`:

```bash
git clone https://codeberg.org/wvhulle/lean-prism
LEAN_DAG_SCHEMA=$PWD/lean-prism/protocol-schema.json cargo install lean-tui
```

Make sure `~/.cargo/bin` is on your `PATH`.

Static Linux binaries are also attached to every [Codeberg release](https://codeberg.org/wvhulle/lean-tui/releases).

## Install `lean-prism` without Nix

`lean-prism serve` replaces `lake serve`. It runs `lake serve` for your project with Lean-Prism loaded as a plugin built for the project's Lean version, so your project needs no changes.

Build it from a clone with `lake build lean-prism lean-server-plugin` and put `.lake/build/bin` on your `PATH`. A source build fetches and builds the plugin for your project's Lean version the first time you open the project, and reports that build to the editor as LSP progress. See [Lean-Prism's supported versions](https://codeberg.org/wvhulle/lean-prism#supported-lean-versions).

If your project already requires `LeanPrism` in its lakefile, `lean-prism serve` warns and falls back to plain `lake serve`. Remove the requirement and the `import LeanPrism` lines to use the plugin.

## Configure your editor

Point your editor's Lean language server at `lean-prism serve`.

### Helix

Recommended, and used by the author.

```toml
# ~/.config/helix/languages.toml or .helix/languages.toml
[language-server.lean-prism]
command = "lean-prism"
args = ["serve"]

[[language]]
name = "lean"
language-servers = ["lean-prism"]
```

Set `editor.lsp.display-progress-messages = true` in `config.toml` to see when the plugin is being built.

### Neovim

Configure `lean-prism serve` as the language server for `.lean` files, see the [Neovim LSP docs](https://neovim.io/doc/user/lsp.html). With [lean.nvim](https://github.com/Julian/lean.nvim), set its server command to `lean-prism serve`.

### Zed

Configure `lean-prism serve` as the Lean language server, see the [Zed docs](https://zed.dev/docs/configuring-languages).

### Emacs

Configure `lean-prism serve` as the server command of your Lean LSP client (eglot or lsp-mode).

### VS Code

The Lean 4 extension starts `lake serve` itself, so add Lean-Prism as a library instead, see [Lean-Prism's library setup](https://codeberg.org/wvhulle/lean-prism#library). You may want to close the built-in infoview.

## Run it

1. Split your terminal: two terminal windows side by side, or one window with `tmux` or `zellij`.
2. Open a Lean file in your editor. The language server starts and, the first time, builds the Lean-Prism plugin for your Lean version, which can take a while.
3. Run `lean-tui` in the other terminal, from any directory. Until it receives a graph, it tells you what it is waiting for and which keys to press in your editor.

## When the view updates

Lean-Prism answers on hover, document highlight, completion and goal requests. Which editor action sends one of those:

| Editor  | Action                                                                                                       |
| ------- | ------------------------------------------------------------------------------------------------------------ |
| Helix   | `space k` (hover) or `space h` (highlight) in normal mode, `ctrl-x` or typing in insert mode. Moving the cursor alone sends nothing. |
| Neovim  | `K` (hover). With lean.nvim the infoview also updates the graph on every cursor move.                        |
| VS Code | Every cursor move: the Lean 4 extension asks for the goal.                                                  |
| Zed     | Hovering the mouse over code.                                                                                |
| Emacs   | Leaving the cursor on code: eldoc asks for hover documentation after a short delay.                          |

## Command-line options

| Option          | Default | Meaning                                                              |
| --------------- | ------- | -------------------------------------------------------------------- |
| `-p`, `--port`  | `9742`  | First TCP port scanned for Lean-Prism workers (the next 10 are scanned too). |
| `-v`, `--verbose` | off   | Verbose logging to `~/.cache/lean-tui.log`.                          |

Each Lean file worker of Lean-Prism listens on its own port. Use `f` in lean-tui to pick another open file.

## Troubleshooting

- **No graph appears**: check that the editor really runs `lean-prism serve` (Helix: `:lsp-restart`, then watch the progress messages) and that you triggered one of the requests above.
- **Errors like "incompatible headers" or missing `.olean` files**: stale build artifacts. Close lean-tui and the editor, run `lake build` (or remove `.lake/build`) and restart.
- **Logs**: `tail -f ~/.cache/lean-tui.log` for lean-tui (run it with `--verbose`). For Helix's LSP client: `tail -f ~/.cache/helix/helix.log`.
