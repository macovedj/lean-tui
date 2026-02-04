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
    time::{interval, Interval},
};

use crate::{
    app::App,
    connection::{discover_workers, ConnectionPool},
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

/// Core event loop handling connection pool, input, and rendering.
async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    initial_port: u16,
) -> Result<()> {
    let mut pool = ConnectionPool::new();
    let mut app = App::new(initial_port);
    let mut event_stream = EventStream::new();
    let mut command_interval = interval(Duration::from_millis(50));
    let mut scan_interval = interval(Duration::from_secs(2));

    while !app.should_exit {
        handle_active_switch(&mut app, &mut pool);
        handle_scan_and_connect(&mut app, &mut pool).await;

        app.update();
        app.connected = pool.has_any_connection();
        if let Some(port) = pool.active_port() {
            app.connected_file_uri = pool.file_uri(port).cloned();
        }
        terminal.draw(|frame| app.render(frame))?;

        handle_events(
            &mut app,
            &mut pool,
            &mut event_stream,
            &mut command_interval,
            &mut scan_interval,
        )
        .await;
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

/// Handle pending worker scan requests and connect to discovered workers.
async fn handle_scan_and_connect(app: &mut App, pool: &mut ConnectionPool) {
    if app.scan_requested {
        app.scan_requested = false;
        let (port_min, port_max) = app.port_range();
        let workers = discover_workers(port_min, port_max).await;

        // Connect to all discovered workers
        pool.connect_all(&workers).await;

        // Update app with discovered workers
        app.set_discovered_workers(workers, pool);
    }
}

/// Perform a periodic worker scan and connect to new workers.
async fn do_periodic_scan(app: &mut App, pool: &mut ConnectionPool) {
    if app.needs_periodic_scan() {
        let (port_min, port_max) = app.port_range();
        let workers = discover_workers(port_min, port_max).await;

        // Connect to all discovered workers
        pool.connect_all(&workers).await;

        // Update app with discovered workers
        app.set_discovered_workers(workers, pool);
    }
}

/// Handle all events: user input, messages from workers, commands, and periodic
/// scans.
async fn handle_events(
    app: &mut App,
    pool: &mut ConnectionPool,
    event_stream: &mut EventStream,
    command_interval: &mut Interval,
    scan_interval: &mut Interval,
) {
    tokio::select! {
        biased;
        Some(Ok(event)) = event_stream.next() => {
            app.handle_event(&event);
        }
        result = pool.recv_any() => {
            match result {
                Ok(Some(pool_msg)) => {
                    app.handle_pool_message(pool_msg.port, pool_msg.message, pool);
                }
                Ok(None) => {}
                Err(e) => app.set_error(e.to_string()),
            }
        }
        _ = command_interval.tick() => {
            for cmd in app.take_commands() {
                let _ = pool.send_to_active(&cmd).await;
            }
        }
        _ = scan_interval.tick() => {
            do_periodic_scan(app, pool).await;
        }
    }
}
