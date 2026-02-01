//! TCP connection to lean-dag server.

use std::{io, time::Duration};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{tcp::OwnedWriteHalf, TcpStream},
    sync::mpsc,
    time::sleep,
};

use super::protocol::{tcp_address, Command, Message};

/// Handle for communicating with lean-dag TCP server.
pub struct Connection {
    /// Receiver for incoming messages from lean-dag.
    pub rx: mpsc::Receiver<Message>,
    /// Sender for outgoing commands to lean-dag.
    pub tx: mpsc::Sender<Command>,
}

/// Spawn a background task that connects to the lean-dag TCP server.
pub fn spawn() -> Connection {
    let (msg_tx, msg_rx) = mpsc::channel::<Message>(16);
    let (cmd_tx, cmd_rx) = mpsc::channel::<Command>(16);

    tokio::spawn(connection_loop(msg_tx, cmd_rx));

    Connection {
        rx: msg_rx,
        tx: cmd_tx,
    }
}

async fn connection_loop(msg_tx: mpsc::Sender<Message>, mut cmd_rx: mpsc::Receiver<Command>) {
    let addr = tcp_address();
    loop {
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                tracing::info!("Connected to lean-dag at {addr}");
                handle_connection(stream, &msg_tx, &mut cmd_rx).await;
                tracing::info!("Disconnected from lean-dag");
            }
            Err(e) => {
                tracing::debug!("Failed to connect to lean-dag at {addr}: {e}");
                sleep(Duration::from_secs(1)).await;
            }
        }
    }
}

async fn handle_connection(
    stream: TcpStream,
    msg_tx: &mpsc::Sender<Message>,
    cmd_rx: &mut mpsc::Receiver<Command>,
) {
    let (reader, mut writer) = stream.into_split();
    let reader = BufReader::new(reader);
    let mut lines = reader.lines();

    loop {
        tokio::select! {
            line_result = lines.next_line() => {
                match handle_incoming_line(line_result, msg_tx).await {
                    Ok(true) => {}
                    Ok(false) | Err(()) => break,
                }
            }
            Some(cmd) = cmd_rx.recv() => {
                if send_command(&mut writer, &cmd).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn handle_incoming_line(
    line_result: Result<Option<String>, io::Error>,
    msg_tx: &mpsc::Sender<Message>,
) -> Result<bool, ()> {
    match line_result {
        Ok(Some(line)) => {
            match serde_json::from_str::<Message>(&line) {
                Ok(msg) => {
                    tracing::debug!("Received message: {:?}", msg.message_type());
                    if msg_tx.send(msg).await.is_err() {
                        return Err(());
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to parse message: {e}");
                }
            }
            Ok(true)
        }
        Ok(None) | Err(_) => Ok(false),
    }
}

async fn send_command(writer: &mut OwnedWriteHalf, cmd: &Command) -> Result<(), ()> {
    let Ok(json) = serde_json::to_string(cmd) else {
        return Ok(());
    };
    let line = format!("{json}\n");
    writer.write_all(line.as_bytes()).await.map_err(|_| ())
}
