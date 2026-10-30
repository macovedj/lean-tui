use std::{env, fs, path::Path, process::Command};

use crate::{
    command_runner::{command_stdout, required_variable, run_command, TaskError},
    directory_files::{copy_writable_file, copy_writable_tree, files_with_suffix, subdirectories},
};

const SCREENSHOTS_COMMIT_MESSAGE: &str = "docs: regenerate screenshots [skip ci]";

const RECORDINGS_BRANCH: &str = "recordings";
const RECORDINGS_STAGING_DIRECTORY: &str = "lean-tui-recordings";

pub fn record_scene(lean_project: &Path, tape: &Path, out: &Path) -> Result<(), TaskError> {
    copy_writable_tree(lean_project, Path::new("."))?;
    let scene = tape.file_stem().unwrap_or_default().to_string_lossy();
    let video_name = format!("{scene}.mp4");
    let webp_name = format!("{scene}.webp");
    run_command(Command::new("vhs").arg(tape))?;
    run_command(Command::new("ffmpeg").args([
        "-loglevel",
        "error",
        "-i",
        &video_name,
        "-vf",
        "mpdecimate",
        "-fps_mode",
        "vfr",
        "-c:v",
        "libwebp_anim",
        "-lossless",
        "0",
        "-q:v",
        "80",
        "-compression_level",
        "6",
        "-loop",
        "0",
        &webp_name,
    ]))?;
    fs::create_dir_all(out)?;
    fs::copy(&webp_name, out.join(&webp_name))?;
    Ok(())
}

fn codeberg_push_url() -> Result<String, TaskError> {
    let forge_url = required_variable("CI_FORGE_URL")?;
    let forge_host = forge_url
        .split_once("://")
        .map_or(forge_url.as_str(), |(_, after_scheme)| after_scheme)
        .split('/')
        .next()
        .unwrap_or_default();
    let token = required_variable("CODEBERG_TOKEN")?;
    let repository = required_variable("CI_REPO")?;
    Ok(format!(
        "https://oauth2:{token}@{forge_host}/{repository}.git"
    ))
}

fn stage_demo_recordings(recorded_demo: &Path, staging: &Path) -> Result<(), TaskError> {
    let demo_directory = staging.join(recorded_demo.file_name().unwrap_or_default());
    fs::create_dir_all(&demo_directory)?;
    files_with_suffix(recorded_demo, ".webp")?
        .iter()
        .try_for_each(|recording| {
            copy_writable_file(
                recording,
                &demo_directory.join(recording.file_name().unwrap_or_default()),
            )
        })?;
    Ok(())
}

fn published_recordings_tree(staging: &Path, push_url: &str) -> Result<Option<String>, TaskError> {
    let remote_branch = format!("refs/heads/{RECORDINGS_BRANCH}");
    if command_stdout(Command::new("git").current_dir(staging).args([
        "ls-remote",
        push_url,
        &remote_branch,
    ]))?
    .trim()
    .is_empty()
    {
        return Ok(None);
    }
    run_command(Command::new("git").current_dir(staging).args([
        "fetch",
        "--quiet",
        "--depth",
        "1",
        push_url,
        &remote_branch,
    ]))?;
    Ok(Some(command_stdout(
        Command::new("git")
            .current_dir(staging)
            .args(["rev-parse", "FETCH_HEAD^{tree}"]),
    )?))
}

pub fn publish_screenshots(recordings: &Path) -> Result<(), TaskError> {
    let recorded_demos = subdirectories(recordings)?;
    let recorded_scene_count = recorded_demos
        .iter()
        .map(|recorded_demo| files_with_suffix(recorded_demo, ".webp").map(|scenes| scenes.len()))
        .sum::<Result<usize, _>>()?;
    if recorded_scene_count == 0 {
        return Err(TaskError::MissingRecordings {
            directory: recordings.to_path_buf(),
        });
    }
    let staging = env::temp_dir().join(RECORDINGS_STAGING_DIRECTORY);
    if staging.exists() {
        fs::remove_dir_all(&staging)?;
    }
    fs::create_dir_all(&staging)?;
    recorded_demos
        .iter()
        .try_for_each(|recorded_demo| stage_demo_recordings(recorded_demo, &staging))?;
    run_command(Command::new("git").current_dir(&staging).args([
        "init",
        "--quiet",
        "--initial-branch",
        RECORDINGS_BRANCH,
    ]))?;
    run_command(
        Command::new("git")
            .current_dir(&staging)
            .args(["add", "--all"]),
    )?;
    run_command(Command::new("git").current_dir(&staging).args([
        "-c",
        "user.name=lean-tui screenshots",
        "-c",
        "user.email=ci@codeberg.org",
        "commit",
        "--quiet",
        "--message",
        SCREENSHOTS_COMMIT_MESSAGE,
    ]))?;
    let push_url = codeberg_push_url()?;
    let staged_tree = command_stdout(
        Command::new("git")
            .current_dir(&staging)
            .args(["rev-parse", "HEAD^{tree}"]),
    )?;
    if published_recordings_tree(&staging, &push_url)?.as_deref() == Some(staged_tree.as_str()) {
        println!("screenshots unchanged");
        return Ok(());
    }
    run_command(Command::new("git").current_dir(&staging).args([
        "push",
        "--force",
        &push_url,
        &format!("HEAD:refs/heads/{RECORDINGS_BRANCH}"),
    ]))
}
