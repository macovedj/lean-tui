//! TCP connection to lean-dag server.

use std::io;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines},
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
};

use super::protocol::{Command, Message};

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
                tracing::debug!("Failed to connect to lean-dag at {addr}: {e}");
                Self::Disconnected
            }
        }
    }

    /// Check if connected.
    pub const fn is_connected(&self) -> bool {
        matches!(self, Self::Connected { .. })
    }

    /// Try to receive a message. Returns None if disconnected or no data ready.
    pub async fn recv(&mut self) -> Option<Message> {
        let Self::Connected { reader, .. } = self else {
            return None;
        };

        match reader.next_line().await {
            Ok(Some(line)) => match serde_json::from_str::<Message>(&line) {
                Ok(msg) => {
                    tracing::debug!("Received message: {:?}", msg.message_type());
                    Some(msg)
                }
                Err(e) => {
                    tracing::warn!("Failed to parse message: {e}");
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
    pub async fn send(&mut self, cmd: &Command) -> io::Result<()> {
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
