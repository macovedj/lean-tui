//! CLI runtime and connection management.

use std::{
    fs,
    io::{stdout, Stdout},
    path::PathBuf,
    process,
    time::Duration,
};

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use futures::StreamExt;
use ratatui::prelude::*;
use tokio::{
    runtime::Builder as RuntimeBuilder,
    time::{interval, sleep, Interval},
};

use crate::{
    app::App,
    connection::{discover_workers, Connection},
    error::Result,
};

/// Standalone TUI infoview for Lean 4 theorem prover
#[derive(Parser)]
#[command(version, about)]
pub struct Args {
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

/// Initialize logging to a file in the cache directory.
pub fn init_logging(verbose: bool) {
    let log_path = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lean-tui.log");

    if let Ok(log_file) = fs::File::create(&log_path) {
        let level = if verbose { "debug" } else { "info" };
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
}

/// Run the application from CLI arguments.
pub fn run_from_args() {
    let args = Args::parse();
    init_logging(args.verbose);

    if let Err(e) = run_blocking(args.port) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

/// Blocking entry point that sets up the tokio runtime.
fn run_blocking(port: u16) -> Result<()> {
    RuntimeBuilder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run(port))
}

/// Main async event loop.
async fn run(port: u16) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = event_loop(&mut terminal, port).await;
    restore_terminal()?;
    result
}

/// Set up the terminal for TUI rendering.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout()))?)
}

/// Restore the terminal to its original state.
fn restore_terminal() -> Result<()> {
    disable_raw_mode()?;
    stdout()
        .execute(DisableMouseCapture)?
        .execute(LeaveAlternateScreen)?;
    Ok(())
}

/// Core event loop handling connection, input, and rendering.
async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    initial_port: u16,
) -> Result<()> {
    let mut conn = Connection::Disconnected;
    let mut app = App::new(initial_port);
    let mut event_stream = EventStream::new();
    let mut command_interval = interval(Duration::from_millis(50));
    let mut current_port = initial_port;

    while !app.should_exit {
        handle_port_switch(&mut app, &mut conn, &mut current_port).await;
        handle_scan_request(&mut app).await;

        app.update();
        app.connected = conn.is_connected();
        terminal.draw(|frame| app.render(frame))?;

        match &mut conn {
            Connection::Disconnected => {
                handle_disconnected(&mut app, &mut conn, &mut event_stream, current_port).await;
            }
            Connection::Connected { .. } => {
                handle_connected(
                    &mut app,
                    &mut conn,
                    &mut event_stream,
                    &mut command_interval,
                )
                .await;
            }
        }
    }

    Ok(())
}

/// Handle pending port switch requests.
async fn handle_port_switch(app: &mut App, conn: &mut Connection, current_port: &mut u16) {
    if let Some(new_port) = app.take_port_switch() {
        if new_port != *current_port {
            tracing::info!("Switching to port {new_port}");
            conn.disconnect();
            app.clear_for_worker_switch();
            *current_port = new_port;
            *conn = Connection::connect(*current_port).await;
        }
    }
}

/// Handle pending worker scan requests.
async fn handle_scan_request(app: &mut App) {
    if app.scan_requested {
        app.scan_requested = false;
        let (port_min, port_max) = app.port_range();
        let workers = discover_workers(port_min, port_max).await;
        app.set_discovered_workers(workers);
    }
}

/// Handle events while disconnected, attempting reconnection.
async fn handle_disconnected(
    app: &mut App,
    conn: &mut Connection,
    event_stream: &mut EventStream,
    current_port: u16,
) {
    tokio::select! {
        biased;
        Some(Ok(event)) = event_stream.next() => {
            app.handle_event(&event);
        }
        () = sleep(Duration::from_secs(1)) => {
            *conn = Connection::connect(current_port).await;
        }
    }
}

/// Handle events while connected, processing messages and commands.
async fn handle_connected(
    app: &mut App,
    conn: &mut Connection,
    event_stream: &mut EventStream,
    command_interval: &mut Interval,
) {
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
