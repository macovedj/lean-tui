//! TUI for displaying Lean proof goals.

mod app;
mod cli;
mod connection;
mod error;
mod modes;
mod protocol;
mod widgets;

fn main() {
    cli::run_from_args();
}
