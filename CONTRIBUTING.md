# Contributing

This is a more in-depth guide for advanced users or contributors.

## Previous Work

For the **VS Code users**: Yes, the official info view endorsed by the Lean organization is written for VS Code. However, I don't like VS Code and vendor lock-in. We should all try to move away from any Microsoft-owned product as soon as possible. Why? Take a look at for example ["Embrace, extend, and extinguish"](https://en.wikipedia.org/wiki/Embrace,_extend,_and_extinguish) but also the recent news.

For the **Neovim users**, yes there existed an info view [Lean plugin for Neovim](https://github.com/Julian/lean.nvim) already, but not yet for the other terminal editors (Zed, Kakoune, Helix, ...) since the Lean plugin relies on Neovim specific features. It is not a generic one.

## How Does It Work?

This is mainly a front for [**lean-prism**](https://codeberg.org/wvhulle/lean-prism). `lean-prism serve` runs `lake serve` with a Lean-Prism server plugin built for the project's Lean version. The plugin chains onto the hover, document highlight, completion and goal requests of Lean's language server and uses Lean's internal APIs to extract proof information (tactic applications, goal transformations, go-to locations) that isn't available through standard LSP. Every file worker serves the graph under the cursor over TCP on its own port in 9742–9842; lean-tui scans the first ports of that range.

The name `lean-prism` comes from showing proof state through different "prisms" or perspectives - the various graph and visualization modes that let you examine your proofs from different angles.

```mermaid
flowchart

subgraph lean-prism-process["Process: lean-prism serve"]
    lake-serve[lake serve]

    subgraph lean-worker[Lean file worker]
        original-lean-lsp[Official Lean.Server]
        plugin[Lean-Prism server plugin]
        tcp-server[TCP server]

        original-lean-lsp -->|chained requests| plugin
        plugin -->|graph under cursor| tcp-server
    end

    lake-serve -->|JSON-RPC| original-lean-lsp
end

subgraph TUI["Process: lean-tui view"]
    tcp-client[TCP Client]
    info-view{Interactive Proof UI}
    ratatui[Ratatui Framework]

    tcp-client <-->|Rust| info-view
    info-view <-->|Rust| ratatui
end

subgraph Editor
    editor-ui{Helix / Neovim / Zed / Emacs}
    editor-lsp-client[Builtin LSP Client]
    editor-ui <--> editor-lsp-client
end

tcp-client <-->|"TCP 9742–9842, JSON"| tcp-server
editor-lsp-client -->|LSP/JSON| lake-serve
```

## Debugging

See [troubleshooting](docs/setup.md#troubleshooting). When reporting a bug, run `lean-tui --verbose` and attach `~/.cache/lean-tui.log` (and your editor's LSP log, for Helix `~/.cache/helix/helix.log`).

## Development

The protocol types are generated from `protocol-schema.json` in lean-prism. `build.rs` reads the file named by `LEAN_DAG_SCHEMA`, which `nix build` and `nix develop` set from the `lean-prism` flake input (lean-prism's `main`). That input is the only pin. To pick up a protocol change:

```bash
nix flake update lean-prism
```

A `cargo` build outside the devshell needs `LEAN_DAG_SCHEMA` pointing at a checkout of lean-prism's `protocol-schema.json`.

The flake is split by concern: `nix/` builds, checks and releases the Rust crate, and `recordings/` holds everything that needs Lean.

Repository tasks are Rust subcommands of the `xtask` crate, run with `cargo xtask <task>` (`cargo xtask --help` lists them). Nix wraps each one with its runtime tools in `nix/xtask-tool.nix`, so CI calls them as `nix run .#<task>`.

`docs/keybindings.md` is generated from `src/keybindings.rs`. After changing a keybinding, run `cargo xtask keybinding-docs`; the `keybinding-docs` flake check fails when the file is out of date.

### Releases

Releases are automated with [release-plz](https://release-plz.dev/) on Woodpecker, so write [conventional commits](https://www.conventionalcommits.org/) (pull requests are checked with `convco`).

- `.woodpecker/checks.yaml` builds the crate, runs the tests and `rustfmt`, and checks that `docs/keybindings.md` is up to date. It never needs Lean.
- `.woodpecker/release.yaml` runs on every push to `main`: it publishes a bumped version to crates.io, tags it and creates the Codeberg release, then opens or updates a release pull request with the next version and changelog. Merging that pull request triggers the next release. `feat`, `fix` and `perf` commits trigger a release; other types only appear in the changelog.
- `.woodpecker/binaries.yaml` builds static musl binaries (`nix build .#dist`) and attaches them to each Codeberg release.

The Woodpecker repository needs the secrets `codeberg_token` (repository write access), `cargo_registry_token` (crates.io publish token for `lean-tui`) and `cachix_auth_token` (push access to `wvhulle.cachix.org`).

### Regenerating Screenshots

The WebP animations in `screenshots/` are recorded with [VHS](https://github.com/charmbracelet/vhs) tapes in `recordings/tapes/`: each `<scene>.tape` opens Helix (with `lean-prism serve` as language server, see `recordings/.helix/`) and `lean-tui` side by side in tmux (shared setup in `open.tape`, shared settings in `common.tape`) and plays the keystrokes. Tapes trigger graph updates with `space h` (document highlight) instead of hover, so no popup covers the code. VHS records MP4, and `cargo xtask record-scene` encodes it to an animated WebP with ffmpeg, dropping duplicate frames (`mpdecimate`) so still moments cost nothing.

```bash
nix build .#recordings && cp --no-preserve=mode result/*.webp screenshots/
```

Tapes jump to absolute line numbers (`:12`), so update them when you edit a sample.

Each tape is its own derivation (`.#recording-semantic_tableau`, `.#recording-data_flow`, ...), so Nix records them in parallel and only re-records tapes whose inputs changed. To add a scene, add a `<scene>.tape` next to the others.

On every push to `main` that touches the crate (`src/`, `build.rs`, `Cargo.*`, `flake.lock`, `nix/package.nix`, `nix/toolchains.nix`) or `recordings/`, `.woodpecker/screenshots.yaml` builds the recordings (and with them the Lean project), pushes them to `wvhulle.cachix.org`, and replaces the files in `screenshots/` on `main` as `docs: regenerate screenshots [skip ci]`. Because unchanged recordings come from the cache, they keep the same bytes and are not committed again. It needs two Woodpecker secrets: `cachix_auth_token` (push access to the `wvhulle` cache) and `codeberg_token` (a Codeberg access token with repository write access).

The Lean projects:

- `recordings/`: plain Lake project without dependencies; the recordings get Lean-Prism from the `lean-prism` flake input through `lean-prism serve`. `recordings/Samples/` holds the Lean files shown in the docs and recordings, `recordings/tapes/` the VHS tapes.
- `tests/lean-dev/`: Lake project for co-developing against a sibling `../lean-prism` checkout (with Mathlib). Its `Samples` links to `recordings/Samples`.
