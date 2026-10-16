use std::{
    env,
    path::Path,
    thread,
    time::{Duration, Instant},
};

use crate::{
    error::{RecordingError, Result},
    terminal::{CursorPosition, Key, Pane, PaneSize},
};

const POLL_INTERVAL: Duration = Duration::from_millis(50);
const QUIET_PERIOD: Duration = Duration::from_millis(500);
const DIVIDER: &str = "│";
const FOCUSED_DIVIDER_STYLE: &str = "\x1b[0;34m";
const UNFOCUSED_DIVIDER_STYLE: &str = "\x1b[0;90m";
const RESET_STYLE: &str = "\x1b[0m";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Editor,
    View,
}

#[derive(Clone, Copy)]
pub struct StageSize {
    pub rows: u16,
    pub cols: u16,
}

impl StageSize {
    const fn editor(self) -> PaneSize {
        PaneSize {
            rows: self.rows,
            cols: (self.cols - 1) / 2,
        }
    }

    const fn view(self) -> PaneSize {
        PaneSize {
            rows: self.rows,
            cols: self.cols - 1 - self.editor().cols,
        }
    }

    const fn view_column(self) -> u16 {
        self.editor().cols + 1
    }
}

pub struct Stage {
    editor: Pane,
    view: Pane,
    focus: Focus,
    size: StageSize,
    wait_timeout: Duration,
}

#[derive(PartialEq, Eq)]
struct Observation {
    editor: Vec<String>,
    view: Vec<String>,
    cursor: Option<CursorPosition>,
}

fn timeout_from_environment() -> Duration {
    env::var("RECORDING_WAIT_TIMEOUT")
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(Duration::from_mins(3), Duration::from_secs)
}

impl Stage {
    pub fn open(lean_file: &str, size: StageSize) -> Result<Self> {
        let working_dir = env::current_dir()?;
        Ok(Self {
            editor: Pane::spawn("hx", &[lean_file], &working_dir, size.editor())?,
            view: Pane::spawn("lean-tui", &[], Path::new(&working_dir), size.view())?,
            focus: Focus::Editor,
            size,
            wait_timeout: timeout_from_environment(),
        })
    }

    pub const fn size(&self) -> StageSize {
        self.size
    }

    pub fn focus(&mut self, focus: Focus) {
        self.focus = focus;
    }

    pub fn send_to_focused(&self, key: &Key<'_>) -> Result<()> {
        self.send(self.focus, key)
    }

    pub fn send(&self, focus: Focus, key: &Key<'_>) -> Result<()> {
        match focus {
            Focus::Editor => self.editor.send(key),
            Focus::View => self.view.send(key),
        }
    }

    pub fn editor_text(&self) -> String {
        self.editor.plain_rows().join("\n")
    }

    pub fn view_text(&self) -> String {
        self.view.plain_rows().join("\n")
    }

    pub fn view_title(&self) -> String {
        self.view.plain_rows().into_iter().next().unwrap_or_default()
    }

    fn observe(&self) -> Observation {
        Observation {
            editor: self.editor.plain_rows(),
            view: self.view.plain_rows(),
            cursor: self.focused_cursor(),
        }
    }

    fn focused_cursor(&self) -> Option<CursorPosition> {
        match self.focus {
            Focus::Editor => self.editor.cursor(),
            Focus::View => self.view.cursor().map(|cursor| CursorPosition {
                row: cursor.row,
                col: cursor.col + self.size.view_column(),
            }),
        }
    }

    fn timeout_error(&self, expectation: &str) -> RecordingError {
        RecordingError::Timeout {
            expectation: expectation.to_owned(),
            editor_screen: self.editor_text(),
            view_screen: self.view_text(),
        }
    }

    pub fn settle_until(&self, expectation: &str, satisfied: impl Fn(&Self) -> bool) -> Result<()> {
        let deadline = Instant::now() + self.wait_timeout;
        let mut previous = self.observe();
        let mut quiet_since = Instant::now();
        loop {
            thread::sleep(POLL_INTERVAL);
            let current = self.observe();
            if current != previous {
                previous = current;
                quiet_since = Instant::now();
            } else if quiet_since.elapsed() >= QUIET_PERIOD && satisfied(self) {
                return Ok(());
            }
            if Instant::now() > deadline {
                return Err(self.timeout_error(expectation));
            }
        }
    }

    pub fn settle(&self) -> Result<()> {
        self.settle_until("the screen to settle", |_| true)
    }

    pub fn retry_until(
        &self,
        expectation: &str,
        satisfied: impl Fn(&Self) -> bool,
        attempt: impl Fn(&Self) -> Result<()>,
    ) -> Result<()> {
        let deadline = Instant::now() + self.wait_timeout;
        while !satisfied(self) {
            if Instant::now() > deadline {
                return Err(self.timeout_error(expectation));
            }
            attempt(self)?;
        }
        Ok(())
    }

    pub fn frame(&self) -> Vec<u8> {
        let divider_style = match self.focus {
            Focus::Editor => FOCUSED_DIVIDER_STYLE,
            Focus::View => UNFOCUSED_DIVIDER_STYLE,
        };
        let view_start = self.size.view_column() + 1;
        let rows = self
            .editor
            .formatted_rows()
            .into_iter()
            .zip(self.view.formatted_rows())
            .enumerate()
            .flat_map(|(index, (editor_row, view_row))| {
                let row = index + 1;
                [
                    format!("\x1b[{row};1H{RESET_STYLE}").into_bytes(),
                    editor_row,
                    format!("{RESET_STYLE}\x1b[{row};{}H{divider_style}{DIVIDER}{RESET_STYLE}", view_start - 1).into_bytes(),
                    view_row,
                    RESET_STYLE.as_bytes().to_vec(),
                ]
            })
            .flatten();
        let cursor = self.focused_cursor().map_or_else(
            || "\x1b[?25l".to_owned(),
            |cursor| format!("\x1b[{};{}H\x1b[?25h", cursor.row + 1, cursor.col + 1),
        );
        b"\x1b[2J"
            .iter()
            .copied()
            .chain(rows)
            .chain(cursor.into_bytes())
            .collect()
    }
}
