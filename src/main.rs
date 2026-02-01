//! TUI for displaying Lean proof goals.

mod app;
mod connection;
mod error;
mod modes;
mod protocol;
mod widgets;

use std::{fs, io::stdout, path::PathBuf, process, time::Duration};

use app::App;
use clap::Parser;
use connection::Connection;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use error::Result;
use futures::StreamExt;
use ratatui::prelude::*;
use tokio::time::{interval, sleep};

/// Standalone TUI infoview for Lean 4 theorem prover
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// TCP port for lean-dag connection
    #[arg(short, long, default_value_t = Args::default().port)]
    port: u16,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            verbose: false,
            port: 9742,
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let log_path = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lean-tui.log");

    if let Ok(log_file) = fs::File::create(&log_path) {
        let level = if args.verbose { "debug" } else { "info" };
        let filter = tracing_subscriber::EnvFilter::from_default_env()
            .add_directive(format!("lean_tui={level}").parse().unwrap());
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(log_file)
            .with_ansi(true)
            .with_target(false)
            .pretty()
            .init();
    }

    if let Err(e) = run(args.port).await {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

async fn run(port: u16) -> Result<()> {
    enable_raw_mode()?;
    stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut conn = Connection::Disconnected;
    let mut app = App::new(port);
    let mut event_stream = EventStream::new();
    let mut command_interval = interval(Duration::from_millis(50));

    while !app.should_exit {
        app.update();
        app.connected = conn.is_connected();
        terminal.draw(|frame| app.render(frame))?;

        match &mut conn {
            Connection::Disconnected => {
                tokio::select! {
                    biased;
                    Some(Ok(event)) = event_stream.next() => {
                        app.handle_event(&event);
                    }
                    () = sleep(Duration::from_secs(1)) => {
                        conn = Connection::connect(port).await;
                    }
                }
            }
            Connection::Connected { .. } => {
                tokio::select! {
                    biased;
                    Some(Ok(event)) = event_stream.next() => {
                        app.handle_event(&event);
                    }
                    Some(msg) = conn.recv() => {
                        app.handle_message(msg);
                    }
                    _ = command_interval.tick() => {
                        for cmd in app.take_commands() {
                            let _ = conn.send(&cmd).await;
                        }
                    }
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
