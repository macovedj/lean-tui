//! TCP connection to lean-dag server.

use std::{io, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    time::timeout,
};

use crate::{
    protocol::{ClientToServerCommand, ServerToClientMessage},
    widgets::worker_selector::WorkerInfo,
};

/// TCP connection state machine.
pub enum Connection {
    Disconnected,
    Connected {
        reader: Lines<BufReader<OwnedReadHalf>>,
        writer: OwnedWriteHalf,
    },
}

impl Connection {
    /// Try to connect to lean-dag server.
    pub async fn connect(port: u16) -> Self {
        let addr = format!("127.0.0.1:{port}");
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                tracing::info!("Connected to lean-dag at {addr}");
                let (reader, writer) = stream.into_split();
                let reader = BufReader::new(reader).lines();
                Self::Connected { reader, writer }
            }
            Err(e) => {
                tracing::trace!("Failed to connect to lean-dag at {addr}: {e}");
                Self::Disconnected
            }
        }
    }

    /// Check if connected.
    pub const fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }

    /// Disconnect from the server.
    pub fn disconnect(&mut self) {
        *self = Self::Disconnected;
    }

    /// Try to receive a message. Returns None if disconnected or no data ready.
    pub async fn recv(&mut self) -> Option<ServerToClientMessage> {
        let Self::Connected { reader, .. } = self else {
            return None;
        };

        match reader.next_line().await {
            Ok(Some(line)) => match serde_json::from_str::<ServerToClientMessage>(&line) {
                Ok(msg) => {
                    tracing::debug!("Received message: {:?}", msg.message_type());
                    Some(msg)
                }
                Err(e) => {
                    tracing::warn!("Failed to parse message: {e}");
                    tracing::debug!("Raw message: {}", &line[..line.len().min(500)]);
                    None
                }
            },
            Ok(None) => {
                tracing::info!("Disconnected from lean-dag");
                *self = Self::Disconnected;
                None
            }
            Err(e) => {
                tracing::warn!("Read error: {e}");
                *self = Self::Disconnected;
                None
            }
        }
    }

    /// Send a command. Returns error if disconnected or write fails.
    pub async fn send(&mut self, cmd: &ClientToServerCommand) -> io::Result<()> {
        let Self::Connected { writer, .. } = self else {
            return Err(io::Error::new(io::ErrorKind::NotConnected, "not connected"));
        };

        let json = serde_json::to_string(cmd)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let line = format!("{json}\n");

        if let Err(e) = writer.write_all(line.as_bytes()).await {
            tracing::warn!("Write error: {e}");
            *self = Self::Disconnected;
            return Err(e);
        }
        Ok(())
    }
}

/// Probe a single port to check if a lean-dag worker is listening.
/// Returns [`WorkerInfo`] if successful.
async fn probe_port(port: u16) -> Option<WorkerInfo> {
    let addr = format!("127.0.0.1:{port}");

    // Try to connect with a short timeout
    let Ok(Ok(stream)) = timeout(Duration::from_millis(100), TcpStream::connect(&addr)).await
    else {
        return None;
    };

    let (reader, _writer) = stream.into_split();
    let mut reader = BufReader::new(reader).lines();

    // Try to read the Connected message with timeout
    let Ok(Ok(Some(line))) = timeout(Duration::from_millis(200), reader.next_line()).await else {
        return None;
    };

    // Parse the message
    let msg: ServerToClientMessage = serde_json::from_str(&line).ok()?;

    match msg {
        ServerToClientMessage::Connected {
            file_uri,
            server_mode,
            ..
        } => Some(WorkerInfo {
            port,
            file_uri,
            server_mode: server_mode.map(|m| format!("{m:?}")),
        }),
        _ => None,
    }
}

/// Scan a range of ports to discover active lean-dag workers.
pub async fn discover_workers(port_min: u16, port_max: u16) -> Vec<WorkerInfo> {
    tracing::info!("Scanning ports {port_min}–{port_max} for workers");

    let mut handles = Vec::new();
    for port in port_min..=port_max {
        handles.push(tokio::spawn(probe_port(port)));
    }

    let mut workers = Vec::new();
    for handle in handles {
        if let Ok(Some(info)) = handle.await {
            tracing::info!("Found worker: {:?}", info);
            workers.push(info);
        }
    }

    // Sort by port
    workers.sort_by_key(|w| w.port);
    workers
}
