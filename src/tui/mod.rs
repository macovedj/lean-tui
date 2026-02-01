//! TUI for displaying Lean proof goals.

pub mod app;
mod connection;
mod error;
mod modes;
pub mod protocol;
mod widgets;

use std::{io::stdout, time::Duration};

use app::App;
use connection::Connection;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, EventStream},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
pub use error::Result;
use futures::StreamExt;
use ratatui::prelude::*;
use tokio::time::{interval, sleep};

pub async fn run(port: u16) -> Result<()> {
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
