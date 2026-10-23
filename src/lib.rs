mod before_after;
mod compact_tree;
mod data_flow;
mod deduction_tree;
mod display_mode;
mod effect_flow;
mod error;
mod event_loop;
mod graph_layout;
mod keybindings;
mod message;
mod model;
mod navigation;
mod protocol;
#[cfg(test)]
mod scenario_replay;
mod selection;
mod tcp_client;
mod theme;
mod update;
mod view;
mod widgets;

use std::{
    fs,
    io::{stdout, Stdout},
    path::PathBuf,
    process,
};

use clap::Parser;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::prelude::*;
use tokio::runtime::Builder as RuntimeBuilder;

use crate::{error::Result, event_loop::run_event_loop};

const SCANNED_PORT_COUNT: u16 = 10;

#[derive(Parser)]
#[command(version, about)]
pub struct Args {
    #[arg(short, long, help = "Enable verbose logging")]
    verbose: bool,

    #[arg(
        short,
        long,
        default_value_t = 9742,
        help = "First TCP port scanned for lean-prism workers"
    )]
    port: u16,
}

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

pub fn run_from_args() {
    let args = Args::parse();
    init_logging(args.verbose);

    if let Err(e) = run_blocking(args.port) {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run_blocking(port: u16) -> Result<()> {
    RuntimeBuilder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(run(port))
}

async fn run(port: u16) -> Result<()> {
    let mut terminal = setup_terminal()?;
    let result = run_event_loop(
        &mut terminal,
        port..=port.saturating_add(SCANNED_PORT_COUNT),
    )
    .await;
    disable_raw_mode()?;
    stdout()
        .execute(DisableMouseCapture)?
        .execute(LeaveAlternateScreen)?;
    result
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;
    Ok(Terminal::new(CrosstermBackend::new(stdout()))?)
}
