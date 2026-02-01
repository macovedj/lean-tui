//! TUI for displaying Lean proof goals.

pub mod app;
mod connection;
mod error;
mod modes;
pub mod protocol;
mod widgets;

use std::{io::stdout, time::Duration};

use app::App;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use futures::StreamExt;
use ratatui::prelude::*;
use tokio::time::sleep;

pub use error::Result;

pub async fn run() -> Result<()> {
    enable_raw_mode()?;
    stdout()
        .execute(EnterAlternateScreen)?
        .execute(EnableMouseCapture)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    let mut conn = connection::spawn();
    let mut app = App::default();
    let mut event_stream = EventStream::new();

    while !app.should_exit {
        app.update();
        terminal.draw(|frame| app.render(frame))?;

        tokio::select! {
            Some(msg) = conn.rx.recv() => {
                app.handle_message(msg);
            }
            Some(Ok(event)) = event_stream.next() => {
                app.handle_event(&event);
            }
            () = sleep(Duration::from_millis(50)) => {
                for cmd in app.take_commands() {
                    let _ = conn.tx.send(cmd).await;
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
