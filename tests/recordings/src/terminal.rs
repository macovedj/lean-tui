use std::{
    io::{Read, Write},
    path::Path,
    sync::{Arc, Mutex, MutexGuard, PoisonError},
    thread,
};

use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};

use crate::error::{RecordingError, Result};

const PRIMARY_DEVICE_ATTRIBUTES_QUERIES: [&[u8]; 2] = [b"\x1b[c", b"\x1b[0c"];
const PRIMARY_DEVICE_ATTRIBUTES_REPLY: &[u8] = b"\x1b[?62;22c";
const CURSOR_POSITION_QUERY: &[u8] = b"\x1b[6n";

#[derive(Clone, Copy)]
pub struct PaneSize {
    pub rows: u16,
    pub cols: u16,
}

pub enum Key<'a> {
    Text(&'a str),
    Enter,
    Escape,
}

impl Key<'_> {
    fn bytes(&self) -> &[u8] {
        match self {
            Self::Text(text) => text.as_bytes(),
            Self::Enter => b"\r",
            Self::Escape => b"\x1b",
        }
    }
}

type SharedWriter = Arc<Mutex<Box<dyn Write + Send>>>;

pub struct Pane {
    parser: Arc<Mutex<vt100::Parser>>,
    writer: SharedWriter,
    child: Box<dyn Child + Send + Sync>,
    _master: Box<dyn MasterPty + Send>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

fn replies_to_queries(output: &[u8], parser: &vt100::Parser) -> Vec<u8> {
    let attributes = PRIMARY_DEVICE_ATTRIBUTES_QUERIES
        .iter()
        .any(|query| contains(output, query))
        .then_some(PRIMARY_DEVICE_ATTRIBUTES_REPLY.to_vec());
    let cursor = contains(output, CURSOR_POSITION_QUERY).then(|| {
        let (row, col) = parser.screen().cursor_position();
        format!("\x1b[{};{}R", row + 1, col + 1).into_bytes()
    });
    attributes.into_iter().chain(cursor).flatten().collect()
}

fn absorb_output(parser: &Mutex<vt100::Parser>, output: &[u8]) -> Vec<u8> {
    let mut parser = lock(parser);
    parser.process(output);
    let replies = replies_to_queries(output, &parser);
    drop(parser);
    replies
}

fn pump_output(mut reader: Box<dyn Read + Send>, parser: &Mutex<vt100::Parser>, writer: &SharedWriter) {
    let mut buffer = [0_u8; 8192];
    while let Ok(read @ 1..) = reader.read(&mut buffer) {
        let replies = absorb_output(parser, &buffer[..read]);
        let delivered = replies.is_empty() || lock(writer).write_all(&replies).is_ok();
        if !delivered {
            return;
        }
    }
}

impl Pane {
    pub fn spawn(program: &str, args: &[&str], working_dir: &Path, size: PaneSize) -> Result<Self> {
        let pair = native_pty_system()
            .openpty(PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(RecordingError::pty)?;
        let mut command = CommandBuilder::new(program);
        command.args(args);
        command.cwd(working_dir);
        command.env("TERM", "xterm-256color");
        command.env("COLORTERM", "truecolor");
        let child = pair.slave.spawn_command(command).map_err(RecordingError::pty)?;
        let reader = pair.master.try_clone_reader().map_err(RecordingError::pty)?;
        let writer: SharedWriter = Arc::new(Mutex::new(pair.master.take_writer().map_err(RecordingError::pty)?));
        let parser = Arc::new(Mutex::new(vt100::Parser::new(size.rows, size.cols, 0)));
        let pump_parser = Arc::clone(&parser);
        let pump_writer = Arc::clone(&writer);
        thread::spawn(move || pump_output(reader, &pump_parser, &pump_writer));
        Ok(Self {
            parser,
            writer,
            child,
            _master: pair.master,
        })
    }

    pub fn send(&self, key: &Key<'_>) -> Result<()> {
        let mut writer = lock(&self.writer);
        writer.write_all(key.bytes())?;
        Ok(writer.flush()?)
    }

    pub fn plain_rows(&self) -> Vec<String> {
        let parser = lock(&self.parser);
        let (_, cols) = parser.screen().size();
        parser.screen().rows(0, cols).collect()
    }

    pub fn formatted_rows(&self) -> Vec<Vec<u8>> {
        let parser = lock(&self.parser);
        let (_, cols) = parser.screen().size();
        parser.screen().rows_formatted(0, cols).collect()
    }

    pub fn cursor(&self) -> Option<CursorPosition> {
        let parser = lock(&self.parser);
        let (row, col) = parser.screen().cursor_position();
        let visible = !parser.screen().hide_cursor();
        drop(parser);
        visible.then_some(CursorPosition { row, col })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CursorPosition {
    pub row: u16,
    pub col: u16,
}

impl Drop for Pane {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
