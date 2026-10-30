use std::{
    fs::File,
    io::{BufRead, BufReader, LineWriter, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    ops::RangeInclusive,
    path::Path,
    time::Duration,
};

use lean_tui::{connected_file_uri, ready_command_json};

use crate::{command_runner::TaskError, directory_files::files_with_suffix};

const WORKER_PORTS: RangeInclusive<u16> = 9742..=9842;
const CONNECT_TIMEOUT: Duration = Duration::from_millis(100);
const GREETING_TIMEOUT: Duration = Duration::from_millis(500);
const REPLAY_FILE_NAME: &str = "replay.jsonl";
const PORTABLE_PROJECT_URI: &str = "file:///project/";

struct WorkerConnection {
    port: u16,
    greeting_line: String,
    project_uri_prefix: String,
    lines: BufReader<TcpStream>,
}

fn demo_file_suffixes(demo_directory: &Path) -> Result<Vec<String>, TaskError> {
    let demo_name = demo_directory
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    Ok(files_with_suffix(demo_directory, ".lean")?
        .iter()
        .filter_map(|lean_file| lean_file.file_name())
        .map(|file_name| format!("/{demo_name}/{}", file_name.to_string_lossy()))
        .collect())
}

fn connect_to_demo_worker(port: u16, demo_file_suffixes: &[String]) -> Option<WorkerConnection> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let stream = TcpStream::connect_timeout(&address, CONNECT_TIMEOUT).ok()?;
    stream.set_read_timeout(Some(GREETING_TIMEOUT)).ok()?;
    let mut lines = BufReader::new(stream);
    let mut greeting_line = String::new();
    lines.read_line(&mut greeting_line).ok()?;
    let file_uri = connected_file_uri(&greeting_line)?;
    let matched_suffix = demo_file_suffixes
        .iter()
        .find(|suffix| file_uri.ends_with(suffix.as_str()))?;
    let project_uri_prefix = format!("{}/", file_uri.strip_suffix(matched_suffix.as_str())?);
    lines.get_ref().set_read_timeout(None).ok()?;
    Some(WorkerConnection {
        port,
        greeting_line,
        project_uri_prefix,
        lines,
    })
}

fn send_ready_command(stream: &mut TcpStream) -> Result<(), TaskError> {
    let ready_json =
        ready_command_json().map_err(|serde_error| TaskError::Io(serde_error.into()))?;
    writeln!(stream, "{ready_json}")?;
    Ok(())
}

pub fn capture_replay(demo_directory: &Path) -> Result<(), TaskError> {
    let demo_file_suffixes = demo_file_suffixes(demo_directory)?;
    let WorkerConnection {
        port,
        greeting_line,
        project_uri_prefix,
        mut lines,
    } = WORKER_PORTS
        .clone()
        .find_map(|port| connect_to_demo_worker(port, &demo_file_suffixes))
        .ok_or_else(|| TaskError::MissingReplayWorker {
            demo_directory: demo_directory.to_path_buf(),
        })?;
    send_ready_command(lines.get_mut())?;
    let replay_path = demo_directory.join(REPLAY_FILE_NAME);
    println!(
        "capturing worker on port {port} into {}, stop with Ctrl-C",
        replay_path.display()
    );
    let mut replay_file = LineWriter::new(File::create(&replay_path)?);
    let portable_line =
        |line: &str| line.replace(project_uri_prefix.as_str(), PORTABLE_PROJECT_URI);
    replay_file.write_all(portable_line(&greeting_line).as_bytes())?;
    lines
        .lines()
        .try_for_each(|line| writeln!(replay_file, "{}", portable_line(&line?)))?;
    Ok(())
}
