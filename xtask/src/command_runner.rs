use std::{
    env,
    error::Error,
    fmt, io,
    path::PathBuf,
    process::{Child, Command, ExitStatus, Output, Stdio},
};

#[derive(Debug)]
pub enum TaskError {
    Io(io::Error),
    MissingVariable {
        name: &'static str,
    },
    CommandFailed {
        program: String,
        status: ExitStatus,
    },
    OutdatedFile {
        path: PathBuf,
        regenerate_command: &'static str,
    },
}

impl fmt::Display for TaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(io_error) => write!(formatter, "{io_error}"),
            Self::MissingVariable { name } => {
                write!(formatter, "environment variable {name} is not set")
            }
            Self::CommandFailed { program, status } => {
                write!(formatter, "{program} failed with {status}")
            }
            Self::OutdatedFile {
                path,
                regenerate_command,
            } => write!(
                formatter,
                "{} is out of date, run `{regenerate_command}`",
                path.display()
            ),
        }
    }
}

impl Error for TaskError {}

impl From<io::Error> for TaskError {
    fn from(io_error: io::Error) -> Self {
        Self::Io(io_error)
    }
}

fn ensure_success(command: &Command, status: ExitStatus) -> Result<(), TaskError> {
    if status.success() {
        Ok(())
    } else {
        Err(TaskError::CommandFailed {
            program: command.get_program().to_string_lossy().into_owned(),
            status,
        })
    }
}

pub fn run_command(command: &mut Command) -> Result<(), TaskError> {
    let status = command.status()?;
    ensure_success(command, status)
}

pub fn command_stdout(command: &mut Command) -> Result<String, TaskError> {
    let Output { status, stdout, .. } = command.stderr(Stdio::inherit()).output()?;
    ensure_success(command, status)?;
    Ok(String::from_utf8_lossy(&stdout).into_owned())
}

pub fn wait_for_child(command: &Command, mut child: Child) -> Result<(), TaskError> {
    let status = child.wait()?;
    ensure_success(command, status)
}

pub fn required_variable(name: &'static str) -> Result<String, TaskError> {
    env::var(name).map_err(|_| TaskError::MissingVariable { name })
}
