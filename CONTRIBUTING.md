# Contributing

This is a more in-depth guide for advanced users or contributors.

## Switch to Standalone Mode

This step is completely optional and might cause issues. For those who don't want to modify their Lean source files, there is an additional mode of operation: standalone. It uses the `lean-dag` binary directly, no imports needed. However, it requires building a binary. This is done with:

```bash
lake build LeanDag/lean-dag
```

This will build a binary `lean-dag` in your project's `.lake` folder that can be used as LSP (instead of `lake serve`) and supports the generation of a graph of your proof state. Configure your editor to use this binary as the Lean language server.

## How Does It Work?

This is mainly a front for [**lean-dag**](github.com/wvhulle/lean-dag/tree/main), a custom LSP server that adds an RPC method on top of the built-in LSP-compliant RPC methods provided by Lean's LSP. The additional RPC methods uses Lean's internal APIs to extract detailed proof information (tactic applications, goal transformations, go-to locations) that isn't available through standard LSP.

The name `lean-dag` comes from Lean [Directed Acyclic Graph (DAG)](https://en.wikipedia.org/wiki/Directed_acyclic_graph) since its main goal is showing your proof state as a graph.

```mermaid
flowchart

subgraph lean-dag-process["Lean RPC Server Process"]
    subgraph lean-worker[Lean RPC Worker]
        lean-runtime((Lean Runtime))
        original-lean-lsp[Official Lean.Server]
        custom-rpc-server[RPC Server with Extension]
        rpc-method[LeanDag RPC Method]

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

TUI <-->|"TCP:9742 JSON"| lean-dag-process
editor-lsp-client -->|LSP/JSON| lake-serve
```

## Debugging

If you see errors in the editor like "incompatible headers", you can try

1. Close both the TUI view and the LSP client and restart.
2. Rebuilding `lean-dag`

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
