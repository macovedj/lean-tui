mod tui;

use std::{fs, path::PathBuf, process};

use clap::Parser;

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
        .join("lean-tui/tui.log");

    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(log_file) = fs::File::create(&log_path) {
        let level = if args.verbose { "trace" } else { "debug" };
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

    if let Err(e) = tui::run(args.port).await {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
