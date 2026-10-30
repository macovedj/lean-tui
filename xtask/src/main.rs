mod ci_builds;
mod command_runner;
mod directory_files;
mod keybinding_docs;
mod recordings;
mod release;
mod replay_capture;

use std::{path::PathBuf, process::ExitCode};

use clap::{Parser, Subcommand};

use crate::{command_runner::TaskError, release::ReleasePlzCommand};

#[derive(Parser)]
struct XtaskCli {
    #[command(subcommand)]
    task: RepoTask,
}

#[derive(Subcommand)]
enum RepoTask {
    RecordScene {
        lean_project: PathBuf,
        tape: PathBuf,
        #[arg(long, default_value = ".")]
        out: PathBuf,
    },
    PublishScreenshots {
        recordings: PathBuf,
    },
    CaptureReplay {
        demo_directory: PathBuf,
    },
    CachedBuild {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        build_args: Vec<String>,
    },
    CheckCommits {
        revision_range: Vec<String>,
    },
    UploadReleaseAssets {
        assets_dir: PathBuf,
    },
    ReleasePlz {
        #[arg(value_enum)]
        release_plz_command: ReleasePlzCommand,
    },
    KeybindingDocs {
        #[arg(long, default_value = "docs/keybindings.md")]
        docs_path: PathBuf,
        #[arg(long)]
        check: bool,
    },
}

fn run_repo_task(task: RepoTask) -> Result<(), TaskError> {
    match task {
        RepoTask::RecordScene {
            lean_project,
            tape,
            out,
        } => recordings::record_scene(&lean_project, &tape, &out),
        RepoTask::PublishScreenshots { recordings } => recordings::publish_screenshots(&recordings),
        RepoTask::CaptureReplay { demo_directory } => {
            replay_capture::capture_replay(&demo_directory)
        }
        RepoTask::CachedBuild { build_args } => ci_builds::cached_build(&build_args),
        RepoTask::CheckCommits { revision_range } => ci_builds::check_commits(&revision_range),
        RepoTask::UploadReleaseAssets { assets_dir } => release::upload_release_assets(&assets_dir),
        RepoTask::ReleasePlz {
            release_plz_command,
        } => release::run_release_plz(release_plz_command),
        RepoTask::KeybindingDocs { docs_path, check } => {
            keybinding_docs::write_keybinding_docs(&docs_path, check)
        }
    }
}

fn main() -> ExitCode {
    match run_repo_task(XtaskCli::parse().task) {
        Ok(()) => ExitCode::SUCCESS,
        Err(task_error) => {
            eprintln!("xtask: {task_error}");
            ExitCode::FAILURE
        }
    }
}
