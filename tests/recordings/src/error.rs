use std::{error::Error, fmt, io, process::ExitStatus, result};

#[derive(Debug)]
pub enum RecordingError {
    Io(io::Error),
    Pty(String),
    Timeout {
        expectation: String,
        editor_screen: String,
        view_screen: String,
    },
    ToolFailed {
        tool: &'static str,
        status: ExitStatus,
    },
    UnknownScene(String),
    MissingEnvironment(&'static str),
}

pub type Result<T> = result::Result<T, RecordingError>;

impl RecordingError {
    pub fn pty(cause: impl fmt::Display) -> Self {
        Self::Pty(cause.to_string())
    }
}

impl fmt::Display for RecordingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(f, "I/O error: {error}"),
            Self::Pty(cause) => write!(f, "pseudo-terminal error: {cause}"),
            Self::Timeout {
                expectation,
                editor_screen,
                view_screen,
            } => write!(
                f,
                "timed out waiting for {expectation}\n--- editor ---\n{editor_screen}\n--- view ---\n{view_screen}"
            ),
            Self::ToolFailed { tool, status } => write!(f, "{tool} failed with {status}"),
            Self::UnknownScene(name) => write!(f, "unknown scene '{name}'"),
            Self::MissingEnvironment(variable) => {
                write!(f, "environment variable {variable} is not set")
            }
        }
    }
}

impl Error for RecordingError {}

impl From<io::Error> for RecordingError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
