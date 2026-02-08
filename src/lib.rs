//! CLI runtime and connection management.

mod app;
mod display_mode;
mod error;
mod protocol;
mod tcp_client;

// Display styles
mod before_after;
mod data_flow;
mod deduction_tree;
mod effect_flow;
mod plain_goal;
mod tactic_tree;

// Shared components
mod graph_layout;
mod shared;

// App-level UI components
mod help_menu;
mod status_bar;
mod welcome;
mod worker_selector;
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
    error::Result,
    tcp_client::{discover_workers, ConnectionPool},
};

#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,

    /// TCP port for lean-prism connection
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
            .add_directive(format!("lean_tui={level}").parse().unwrap_or_default());
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
    disable_raw_mode()?;
    stdout()
        .execute(DisableMouseCapture)?
        .execute(LeaveAlternateScreen)?;
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

/// Core event loop handling connection pool, input, and rendering.
async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    initial_port: u16,
) -> Result<()> {
    let mut pool = ConnectionPool::new();
    let mut app = App::new(initial_port);
    let mut event_stream = EventStream::new();
    let mut scan_interval = interval(Duration::from_secs(2));

    while !app.should_exit {
        handle_active_switch(&mut app, &mut pool);
        scan_and_connect(&mut app, &mut pool).await;

        if let Some(port) = pool.take_reconnected_port() {
            tracing::info!("Worker on port {port} reconnected, clearing state");
            app.clear_for_worker_switch(port);
        }

        drain_messages(&mut app, &mut pool).await;

        app.update();
        // Update connection state - active_port is set when we have a valid connection
        if let Some(port) = pool.active_port() {
            app.connected = true;
            app.connected_file_uri = pool.file_uri(port).cloned();
        } else {
            app.connected = false;
            app.connected_file_uri = None;
        }
        terminal.draw(|frame| app.render(frame))?;

        handle_events(&mut app, &mut pool, &mut event_stream, &mut scan_interval).await;
    }

    Ok(())
}

/// Handle pending active port switch requests.
fn handle_active_switch(app: &mut App, pool: &mut ConnectionPool) {
    if let Some(new_port) = app.take_port_switch() {
        if Some(new_port) != pool.active_port() {
            tracing::info!("Switching active to port {new_port}");
            pool.set_active(new_port);
            app.clear_for_worker_switch(new_port);
        }
    }
}

/// Drain all available messages from the connection pool.
async fn drain_messages(app: &mut App, pool: &mut ConnectionPool) {
    while let Ok(Some(pool_msg)) = pool.recv_any().await {
        app.handle_pool_message(pool_msg.port, pool_msg.message, pool);
    }
}

/// Scan for workers and connect if a scan is requested.
async fn scan_and_connect(app: &mut App, pool: &mut ConnectionPool) {
    if !app.scan_requested {
        return;
    }
    app.scan_requested = false;
    let (port_min, port_max) = app.port_range();
    let workers = discover_workers(port_min, port_max).await;
    pool.connect_all(&workers).await;
    app.set_discovered_workers(workers, pool);
}

/// Handle all events: user input, messages from workers, and periodic scans.
async fn handle_events(
    app: &mut App,
    pool: &mut ConnectionPool,
    event_stream: &mut EventStream,
    scan_interval: &mut Interval,
) {
    // Wait for user input, scan interval, or a short timeout for message polling.
    // The short sleep ensures we don't busy-loop but still check for messages
    // frequently.
    tokio::select! {
        Some(Ok(event)) = event_stream.next() => {
            app.handle_event(&event);
        }
        _ = scan_interval.tick() => {
            app.scan_requested = true;
        }
        () = sleep(Duration::from_millis(16)) => {
            // ~60fps polling for network messages
        }
    }

    // Process any messages that arrived
    loop {
        match pool.recv_any().await {
            Ok(Some(pool_msg)) => {
                app.handle_pool_message(pool_msg.port, pool_msg.message, pool);
            }
            Ok(None) => break,
            Err(e) => {
                app.set_error(e.to_string());
                break;
            }
        }
    }

    // Send any queued commands
    for cmd in app.take_commands() {
        let _ = pool.send_to_active(&cmd).await;
    }
}
