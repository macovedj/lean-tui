//! TUI for displaying Lean proof goals.

pub mod app;
mod modes;
mod widgets;

use std::{io::stdout, time::Duration};

use app::App;
use async_lsp::lsp_types::{DidOpenTextDocumentParams, Position, TextDocumentItem, Url};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use futures::StreamExt;
use ratatui::prelude::*;
use tokio::time::sleep;

use crate::{
    error::{LspError, Result},
    lean_rpc::RpcClient,
    tui_ipc::{spawn_socket_handler, Message},
};

/// State for tracking open documents in the RPC client.
struct RpcState {
    client: RpcClient,
    /// Currently open document URI (if any).
    open_uri: Option<Url>,
}

impl RpcState {
    async fn new() -> Result<Self> {
        tracing::info!("[TUI] Spawning RPC client (library mode)");
        let client = RpcClient::new(false)
            .await
            .map_err(crate::error::Error::Lsp)?;
        Ok(Self {
            client,
            open_uri: None,
        })
    }

    /// Ensure the document is open, opening it if necessary.
    async fn ensure_open(&mut self, uri: &Url) -> Result<()> {
        if self.open_uri.as_ref() == Some(uri) {
            return Ok(());
        }

        // Read file content
        let path = uri.to_file_path().map_err(|_| {
            crate::error::Error::Lsp(LspError::RpcError {
                code: None,
                message: "Invalid file URI".into(),
            })
        })?;
        let content = tokio::fs::read_to_string(&path).await?;

        tracing::info!("[TUI] Opening document: {}", uri);
        self.client
            .did_open(DidOpenTextDocumentParams {
                text_document: TextDocumentItem {
                    uri: uri.clone(),
                    language_id: "lean4".to_string(),
                    version: 0,
                    text: content,
                },
            })
            .await
            .map_err(crate::error::Error::Lsp)?;

        self.open_uri = Some(uri.clone());
        Ok(())
    }

    /// Fetch proof DAG at the given position.
    async fn get_proof_dag(
        &mut self,
        uri: &Url,
        position: Position,
    ) -> Result<Option<crate::lean_rpc::ProofDag>> {
        self.ensure_open(uri).await?;
        self.client
            .get_proof_dag(uri, position, "tree")
            .await
            .map_err(crate::error::Error::Lsp)
    }
}

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut socket = spawn_socket_handler();
    let mut app = App::default();
    let mut event_stream = EventStream::new();

    // Spawn RPC client for fetching proof DAGs
    let mut rpc_state = match RpcState::new().await {
        Ok(state) => Some(state),
        Err(e) => {
            tracing::warn!("[TUI] Failed to create RPC client: {}. Proof DAG fetching disabled.", e);
            None
        }
    };

    while !app.should_exit {
        app.update();
        terminal.draw(|frame| app.render(frame))?;

        tokio::select! {
            Some(msg) = socket.rx.recv() => {
                // Handle cursor messages by fetching proof DAG
                if let Message::Cursor(ref cursor) = msg {
                    if let Some(ref mut rpc) = rpc_state {
                        let position = Position {
                            line: cursor.position.line,
                            character: cursor.position.character,
                        };
                        match rpc.get_proof_dag(&cursor.uri, position).await {
                            Ok(proof_dag) => {
                                // Send proof DAG to app
                                let dag_msg = Message::ProofDag {
                                    uri: cursor.uri.clone(),
                                    position: cursor.position,
                                    proof_dag,
                                };
                                app.handle_message(dag_msg);
                            }
                            Err(e) => {
                                tracing::warn!("[TUI] Failed to fetch proof DAG: {}", e);
                                app.handle_message(Message::Error {
                                    error: format!("Failed to fetch proof DAG: {}", e),
                                });
                            }
                        }
                    }
                }
                // Also pass the original message to app
                app.handle_message(msg);
            }
            Some(Ok(event)) = event_stream.next() => {
                app.handle_event(&event);
            }
            () = sleep(Duration::from_millis(50)) => {
                for cmd in app.take_commands() {
                    let _ = socket.tx.send(cmd).await;
                }
            }
        }
    }

    disable_raw_mode()?;
    stdout()
        .execute(DisableMouseCapture)?
        .execute(LeaveAlternateScreen)?;

    Ok(())
}
