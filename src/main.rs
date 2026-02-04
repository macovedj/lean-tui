//! TUI for displaying Lean proof goals.

mod app;
mod cli;
mod connection;
mod display_mode;
mod error;
mod protocol;

// Display styles
mod before_after;
mod data_flow;
mod deduction_tree;
mod effect_flow;
mod plain_goal;
mod tactic_tree;

// Shared components
mod graph;
mod shared;

// App-level UI components
mod help_menu;
mod status_bar;
mod welcome;
mod worker_selector;

fn main() {
    cli::run_from_args();
}
