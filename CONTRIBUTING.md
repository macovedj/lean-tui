# Contributing

This is a more in-depth guide for advanced users or contributors.

## Previous Work

For the **VS Code users**: Yes, the official info view endorsed by the Lean organization is written for VS Code. However, I don't like VS Code and vendor lock-in. We should all try to move away from any Microsoft-owned product as soon as possible. Why? Take a look at for example ["Embrace, extend, and extinguish"](https://en.wikipedia.org/wiki/Embrace,_extend,_and_extinguish) but also the recent news.

For the **Neovim users**, yes there existed an info view [Lean plugin for Neovim](https://github.com/Julian/lean.nvim) already, but not yet for the other terminal editors (Zed, Kakoune, Helix, ...) since the Lean plugin relies on Neovim specific features. It is not a generic one.

## How Does It Work?

This is mainly a front for [**lean-prism**](https://codeberg.org/wvhulle/lean-prism), a custom LSP server that adds an RPC method on top of the built-in LSP-compliant RPC methods provided by Lean's LSP. The additional RPC methods uses Lean's internal APIs to extract detailed proof information (tactic applications, goal transformations, go-to locations) that isn't available through standard LSP.

The name `lean-prism` comes from showing proof state through different "prisms" or perspectives - the various graph and visualization modes that let you examine your proofs from different angles.

```mermaid
flowchart

subgraph lean-prism-process["Lean RPC Server Process"]
    subgraph lean-worker[Lean RPC Worker]
        lean-runtime((Lean Runtime))
        original-lean-lsp[Official Lean.Server]
        custom-rpc-server[RPC Server with Extension]
        rpc-method[LeanPrism RPC Method]

        rpc-method <-->|Lean| custom-rpc-server
        lean-runtime <-->|Lean| rpc-method
        original-lean-lsp --> lean-runtime
        custom-rpc-server <-->|Lean| original-lean-lsp
    end

    lake-serve[lake serve]
    lake-serve -->|JSON-RPC| custom-rpc-server
end

subgraph TUI["Process: lean-tui view"]
    tcp-client[TCP Client]
    info-view{Interactive Proof UI}
    ratatui[Ratatui Framework]

    tcp-client <-->|Rust| info-view
    info-view <-->|Rust| ratatui
end

subgraph Editor
    editor-ui{NeoVim / Helix / Zed}
    editor-lsp-client[Builtin LSP Client]
    editor-ui <--> editor-lsp-client
end

TUI <-->|"TCP:9742 JSON"| lean-prism-process
editor-lsp-client -->|LSP/JSON| lake-serve
```

## Debugging

If you see errors in the editor like "incompatible headers", you can try

1. Close both the TUI view and the LSP client and restart.
2. Rebuilding `lean-prism`

If that does not help, follow along with the logs while reproducing the bug (and paste the output in a bug report):

```bash
tail -f ~/.cache/lean-tui.log # TUI front-end (for debugging the Ratatui-side)
```

Some editors also have debug logs for the LSP client. For Helix:

```bash
tail -f ~/.cache/helix/helix.log
```

## Development

Don't forget to update the commit hash in `build.rs` when the JSON schema changes upstream.

### Regenerating Screenshots

The GIFs in `screenshots/` are recorded with [VHS](https://github.com/charmbracelet/vhs) tapes in `tests/recordings/`: each `<scene>.tape` opens Helix and `lean-tui` side by side in tmux (shared setup in `open.tape`, shared settings in `common.tape`) and plays the keystrokes.

```bash
nix build .#recordings && cp --no-preserve=mode result/*.gif screenshots/
```

Each tape is its own derivation (`.#recording-semantic_tableau`, `.#recording-data_flow`, ...), so Nix records them in parallel and only re-records tapes whose inputs changed. To add a scene, add a `<scene>.tape` next to the others.

Woodpecker on Codeberg runs `nix flake check` on every push and pull request (`.woodpecker/check.yaml`). On every push to `main`, `.woodpecker/screenshots.yaml` builds the recordings, pushes them to `wvhulle.cachix.org`, and commits changed GIFs to `main` as `docs: regenerate screenshots [skip ci]`. Because unchanged recordings come from the cache, they keep the same bytes and are not committed again. It needs two Woodpecker secrets: `cachix_auth_token` (push access to the `wvhulle` cache) and `codeberg_token` (a Codeberg access token with repository write access).

The test projects live in `tests/`:

- `tests/samples/`: the Lean files shown in the README and recordings.
- `tests/lean-dev/`: Lake project for co-developing against a sibling `../lean-prism` checkout (with Mathlib).
- `tests/recordings/`: Lake project with `LeanPrism` pinned in `lake-manifest.json` (update with `lake update` inside that directory), plus the tapes; the flake also takes the protocol schema for the Nix build of `lean-tui` from that revision.
