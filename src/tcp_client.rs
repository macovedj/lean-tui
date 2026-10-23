use std::{io, ops::RangeInclusive, time::Duration};

use futures::future::join_all;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{tcp::OwnedWriteHalf, TcpStream},
    select,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    time::{sleep, timeout},
};

use crate::{
    message::AppMessage,
    protocol::{ClientToServerCommand, ServerToClientMessage},
};

const RECONNECT_DELAY: Duration = Duration::from_secs(2);
const SCAN_INTERVAL: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct WorkerInfo {
    pub port: u16,
    pub file_uri: Option<String>,
    pub server_mode: Option<String>,
    pub editor_name: Option<String>,
}

impl WorkerInfo {
    pub fn filename(&self) -> Option<&str> {
        self.file_uri
            .as_ref()
            .and_then(|uri| uri.rsplit('/').next())
    }

    pub fn display_label(&self) -> String {
        self.file_uri.as_ref().map_or_else(
            || format!("Worker on port {}", self.port),
            |uri| {
                let filename = self.filename().unwrap_or(uri);
                format!("{filename} (port {})", self.port)
            },
        )
    }
}

async fn probe_port(port: u16) -> Option<WorkerInfo> {
    let addr = format!("127.0.0.1:{port}");

    let Ok(Ok(stream)) = timeout(Duration::from_millis(100), TcpStream::connect(&addr)).await
    else {
        return None;
    };

    let (reader, _writer) = stream.into_split();
    let mut reader = BufReader::new(reader).lines();

    let Ok(Ok(Some(line))) = timeout(Duration::from_millis(200), reader.next_line()).await else {
        return None;
    };

    match serde_json::from_str(&line).ok()? {
        ServerToClientMessage::Connected {
            file_uri,
            server_mode,
            editor_name,
            ..
        } => Some(WorkerInfo {
            port,
            file_uri,
            server_mode: server_mode.map(|m| format!("{m:?}")),
            editor_name,
        }),
        _ => None,
    }
}

pub async fn discover_workers(port_min: u16, port_max: u16) -> Vec<WorkerInfo> {
    tracing::info!("Scanning ports {port_min}–{port_max} for workers");

    let handles: Vec<_> = (port_min..=port_max)
        .map(|port| tokio::spawn(probe_port(port)))
        .collect();

    let mut workers: Vec<_> = join_all(handles)
        .await
        .into_iter()
        .filter_map(|result| result.ok().flatten())
        .inspect(|info| tracing::info!("Found worker: {:?}", info))
        .collect();

    workers.sort_by_key(|w| w.port);
    workers
}

pub async fn scan_workers_periodically(
    port_range: RangeInclusive<u16>,
    output: UnboundedSender<AppMessage>,
) {
    loop {
        let workers = discover_workers(*port_range.start(), *port_range.end()).await;
        if output
            .send(AppMessage::WorkersDiscovered { workers })
            .is_err()
        {
            return;
        }
        sleep(SCAN_INTERVAL).await;
    }
}

fn parse_worker_line(port: u16, line: &str) -> AppMessage {
    match serde_json::from_str::<ServerToClientMessage>(line) {
        Ok(message) => {
            tracing::debug!("Received message: {:?}", message.message_type());
            AppMessage::WorkerSent { port, message }
        }
        Err(error) => {
            let raw_preview: String = line.chars().take(500).collect();
            tracing::warn!("Failed to parse message: {error}");
            AppMessage::WorkerSentInvalid {
                port,
                error: format!("Protocol error: {error}\n\nRaw message preview:\n{raw_preview}"),
            }
        }
    }
}

async fn write_command(
    writer: &mut OwnedWriteHalf,
    command: &ClientToServerCommand,
) -> io::Result<()> {
    let json = serde_json::to_string(command)?;
    writer.write_all(format!("{json}\n").as_bytes()).await
}

async fn exchange_with_worker(port: u16, stream: TcpStream, output: &UnboundedSender<AppMessage>) {
    let (reader, mut writer) = stream.into_split();
    let (commands, mut pending_commands) = unbounded_channel();
    let _ = commands.send(ClientToServerCommand::Ready);
    if output
        .send(AppMessage::WorkerConnected { port, commands })
        .is_err()
    {
        return;
    }

    let mut lines = BufReader::new(reader).lines();
    loop {
        select! {
            line = lines.next_line() => match line {
                Ok(Some(line)) => {
                    if output.send(parse_worker_line(port, &line)).is_err() {
                        return;
                    }
                }
                Ok(None) | Err(_) => break,
            },
            Some(command) = pending_commands.recv() => {
                if let Err(error) = write_command(&mut writer, &command).await {
                    tracing::warn!("Write error on port {port}: {error}");
                    break;
                }
            }
        }
    }
    tracing::info!("Worker on port {port} disconnected");
    let _ = output.send(AppMessage::WorkerDisconnected { port });
}

pub async fn stay_connected_to_worker(port: u16, output: UnboundedSender<AppMessage>) {
    loop {
        match TcpStream::connect(("127.0.0.1", port)).await {
            Ok(stream) => {
                tracing::info!("Connected to worker on port {port}");
                exchange_with_worker(port, stream, &output).await;
            }
            Err(error) => tracing::trace!("Failed to connect to port {port}: {error}"),
        }
        if output.is_closed() {
            return;
        }
        sleep(RECONNECT_DELAY).await;
    }
}
