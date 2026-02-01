mod tui;

use std::{fs, path::PathBuf, process};

#[tokio::main]
async fn main() {
    let log_path = dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("lean-tui/tui.log");

    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(log_file) = fs::File::create(&log_path) {
        let filter = tracing_subscriber::EnvFilter::from_default_env().add_directive(
            "lean_tui=debug"
                .parse()
                .unwrap_or_else(|_| "debug".parse().unwrap()),
        );
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_writer(log_file)
            .with_ansi(true)
            .with_target(false)
            .pretty()
            .init();
    }

    if let Err(e) = tui::run().await {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
