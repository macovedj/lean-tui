use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    command_runner::{command_stdout, required_variable, run_command, TaskError},
    directory_files::{copy_writable_file, copy_writable_tree, files_with_suffix, regular_files},
};

const SCREENSHOTS_COMMIT_MESSAGE: &str = "docs: regenerate screenshots [skip ci]";

pub fn record_scene(lean_project: &Path, scene: &str, out: &Path) -> Result<(), TaskError> {
    copy_writable_tree(lean_project, Path::new("."))?;
    let video_name = format!("{scene}.mp4");
    let webp_name = format!("{scene}.webp");
    run_command(Command::new("vhs").arg(format!("{scene}.tape")))?;
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

pub fn publish_screenshots(recordings: &Path) -> Result<(), TaskError> {
    let repository_root = PathBuf::from(
        command_stdout(Command::new("git").args(["rev-parse", "--show-toplevel"]))?.trim(),
    );
    let screenshots = repository_root.join("screenshots");
    let recorded_scenes = files_with_suffix(recordings, ".webp")?;
    if recorded_scenes.is_empty() {
        return Err(TaskError::MissingRecordings {
            directory: recordings.to_path_buf(),
        });
    }
    regular_files(&screenshots)?
        .iter()
        .try_for_each(fs::remove_file)?;
    recorded_scenes.iter().try_for_each(|recording| {
        copy_writable_file(
            recording,
            &screenshots.join(recording.file_name().unwrap_or_default()),
        )
    })?;
    if command_stdout(
        Command::new("git")
            .args(["status", "--porcelain", "--"])
            .arg(&screenshots),
    )?
    .is_empty()
    {
        println!("screenshots unchanged");
        return Ok(());
    }
    run_command(Command::new("git").args(["add", "--"]).arg(&screenshots))?;
    run_command(Command::new("git").args([
        "-c",
        "user.name=lean-tui screenshots",
        "-c",
        "user.email=ci@codeberg.org",
        "commit",
        "--message",
        SCREENSHOTS_COMMIT_MESSAGE,
    ]))?;
    let push_url = codeberg_push_url()?;
    run_command(Command::new("git").args(["fetch", &push_url, "main"]))?;
    run_command(Command::new("git").args(["rebase", "FETCH_HEAD"]))?;
    run_command(Command::new("git").args(["push", &push_url, "HEAD:refs/heads/main"]))
}
