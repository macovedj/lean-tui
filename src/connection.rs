//! TCP connection to lean-dag server.

use std::{collections::HashMap, io, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
    time::timeout,
};

use crate::{
    error::{Error, Result},
    protocol::{ClientToServerCommand, ServerToClientMessage},
    shared::worker_selector::WorkerInfo,
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

    /// Try to receive a message. Returns `Ok(None)` if disconnected or no data
    /// ready. Returns `Err` on parse errors to allow the UI to display
    /// them.
    pub async fn recv(&mut self) -> Result<Option<ServerToClientMessage>> {
        let Self::Connected { reader, .. } = self else {
            return Ok(None);
        };

        match reader.next_line().await {
            Ok(Some(line)) => match serde_json::from_str::<ServerToClientMessage>(&line) {
                Ok(msg) => {
                    tracing::debug!("Received message: {:?}", msg.message_type());
                    Ok(Some(msg))
                }
                Err(e) => {
                    let raw_preview: String = line.chars().take(500).collect();
                    tracing::warn!("Failed to parse message: {e}");
                    tracing::debug!("Raw message: {raw_preview}");
                    Err(Error::ProtocolParse {
                        message: e.to_string(),
                        raw_preview,
                    })
                }
            },
            Ok(None) => {
                tracing::info!("Disconnected from lean-dag");
                *self = Self::Disconnected;
                Ok(None)
            }
            Err(e) => {
                tracing::warn!("Read error: {e}");
                *self = Self::Disconnected;
                Err(Error::Io(e))
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

/// A message received from a specific worker.
pub struct PoolMessage {
    pub port: u16,
    pub message: ServerToClientMessage,
}

/// Pool of connections to multiple lean-dag workers.
pub struct ConnectionPool {
    connections: HashMap<u16, Connection>,
    port_to_uri: HashMap<u16, String>,
    active_port: Option<u16>,
}

impl ConnectionPool {
    /// Create an empty connection pool.
    pub fn new() -> Self {
        Self {
            connections: HashMap::new(),
            port_to_uri: HashMap::new(),
            active_port: None,
        }
    }

    /// Get the currently active port.
    pub fn active_port(&self) -> Option<u16> {
        self.active_port
    }

    /// Set the active port.
    pub fn set_active(&mut self, port: u16) {
        if self.connections.contains_key(&port) {
            self.active_port = Some(port);
        }
    }

    /// Check if connected to at least one worker.
    pub fn has_any_connection(&self) -> bool {
        self.connections.values().any(Connection::is_connected)
    }

    /// Check if connected to a specific port.
    pub fn is_connected(&self, port: u16) -> bool {
        self.connections
            .get(&port)
            .is_some_and(Connection::is_connected)
    }

    /// Get the file URI for a port.
    pub fn file_uri(&self, port: u16) -> Option<&String> {
        self.port_to_uri.get(&port)
    }

    /// Find port by file URI.
    #[allow(dead_code)]
    pub fn port_for_uri(&self, uri: &str) -> Option<u16> {
        self.port_to_uri
            .iter()
            .find(|(_, u)| u.as_str() == uri)
            .map(|(&port, _)| port)
    }

    /// Get list of connected ports.
    pub fn connected_ports(&self) -> Vec<u16> {
        self.connections
            .iter()
            .filter(|(_, c)| c.is_connected())
            .map(|(&port, _)| port)
            .collect()
    }

    /// Connect to a worker if not already connected.
    pub async fn connect(&mut self, worker: &WorkerInfo) {
        if self.is_connected(worker.port) {
            return;
        }

        let conn = Connection::connect(worker.port).await;
        if conn.is_connected() {
            self.connections.insert(worker.port, conn);
            if let Some(uri) = &worker.file_uri {
                self.port_to_uri.insert(worker.port, uri.clone());
            }
            // Auto-activate first connection
            if self.active_port.is_none() {
                self.active_port = Some(worker.port);
            }
        }
    }

    /// Connect to all workers in the list.
    pub async fn connect_all(&mut self, workers: &[WorkerInfo]) {
        for worker in workers {
            self.connect(worker).await;
        }
    }

    /// Disconnect from a specific worker.
    pub fn disconnect(&mut self, port: u16) {
        if let Some(conn) = self.connections.get_mut(&port) {
            conn.disconnect();
        }
        self.connections.remove(&port);
        self.port_to_uri.remove(&port);

        // If we disconnected the active port, switch to another
        if self.active_port == Some(port) {
            self.active_port = self.connected_ports().first().copied();
        }
    }

    /// Disconnect all workers.
    #[allow(dead_code)]
    pub fn disconnect_all(&mut self) {
        for conn in self.connections.values_mut() {
            conn.disconnect();
        }
        self.connections.clear();
        self.port_to_uri.clear();
        self.active_port = None;
    }

    /// Try to receive a message from any connected worker.
    /// Returns the port and message if available.
    pub async fn recv_any(&mut self) -> Result<Option<PoolMessage>> {
        if self.connections.is_empty() {
            return Ok(None);
        }

        let ports: Vec<u16> = self.connections.keys().copied().collect();

        for port in ports {
            match self.try_recv_from(port).await {
                Ok(Some(msg)) => return Ok(Some(PoolMessage { port, message: msg })),
                Ok(None) => continue,
                Err(e) => return Err(e),
            }
        }

        self.cleanup_disconnected();
        Ok(None)
    }

    /// Try to receive from a specific port with a short timeout.
    async fn try_recv_from(&mut self, port: u16) -> Result<Option<ServerToClientMessage>> {
        let Some(conn) = self.connections.get_mut(&port) else {
            return Ok(None);
        };

        if !conn.is_connected() {
            return Ok(None);
        }

        match timeout(Duration::from_millis(1), conn.recv()).await {
            Ok(Ok(Some(msg))) => Ok(Some(msg)),
            Ok(Ok(None)) => {
                tracing::info!("Worker on port {port} disconnected");
                Ok(None)
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Ok(None), // Timeout - no data ready
        }
    }

    /// Remove disconnected connections from the pool.
    fn cleanup_disconnected(&mut self) {
        let disconnected: Vec<u16> = self
            .connections
            .iter()
            .filter(|(_, c)| !c.is_connected())
            .map(|(&p, _)| p)
            .collect();

        for port in disconnected {
            self.disconnect(port);
        }
    }

    /// Send a command to the active worker.
    pub async fn send_to_active(&mut self, cmd: &ClientToServerCommand) -> io::Result<()> {
        let Some(port) = self.active_port else {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "no active connection",
            ));
        };

        let Some(conn) = self.connections.get_mut(&port) else {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "active connection not found",
            ));
        };

        conn.send(cmd).await
    }

    /// Send a command to a specific worker.
    #[allow(dead_code)]
    pub async fn send_to(&mut self, port: u16, cmd: &ClientToServerCommand) -> io::Result<()> {
        let Some(conn) = self.connections.get_mut(&port) else {
            return Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "connection not found",
            ));
        };

        conn.send(cmd).await
    }
}

impl Default for ConnectionPool {
    fn default() -> Self {
        Self::new()
    }
}
