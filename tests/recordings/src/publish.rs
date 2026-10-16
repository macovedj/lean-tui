use std::{
    env, fs,
    path::Path,
    process::{Command, Output},
};

use crate::error::{RecordingError, Result};

const COMMIT_MESSAGE: &str = "docs: regenerate screenshots [skip ci]";

fn environment(variable: &'static str) -> Result<String> {
    env::var(variable).map_err(|_| RecordingError::MissingEnvironment(variable))
}

fn git(args: &[&str]) -> Result<Output> {
    let output = Command::new("git").args(args).output()?;
    if output.status.success() {
        Ok(output)
    } else {
        Err(RecordingError::ToolFailed {
            tool: "git",
            status: output.status,
        })
    }
}

fn copy_gifs(recordings: &Path, screenshots: &Path) -> Result<()> {
    fs::read_dir(recordings)?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "gif"))
        .try_for_each(|gif| {
            let target = screenshots.join(gif.file_name().unwrap_or_default());
            fs::write(target, fs::read(&gif)?)
        })?;
    Ok(())
}

fn push_url() -> Result<String> {
    let forge_url = environment("CI_FORGE_URL")?;
    let forge_host = forge_url
        .trim_start_matches("https://")
        .trim_end_matches('/');
    let token = environment("CODEBERG_TOKEN")?;
    let repository = environment("CI_REPO")?;
    Ok(format!("https://oauth2:{token}@{forge_host}/{repository}.git"))
}

pub fn publish(recordings: &Path, screenshots: &Path) -> Result<()> {
    copy_gifs(recordings, screenshots)?;
    let screenshots_arg = screenshots.to_string_lossy();
    let status = git(&["status", "--porcelain", "--", &screenshots_arg])?;
    if status.stdout.is_empty() {
        println!("screenshots unchanged");
        return Ok(());
    }
    git(&["add", "--", &screenshots_arg])?;
    git(&[
        "-c",
        "user.name=lean-tui screenshots",
        "-c",
        "user.email=ci@codeberg.org",
        "commit",
        "--message",
        COMMIT_MESSAGE,
    ])?;
    let remote = push_url()?;
    git(&["fetch", &remote, "main"])?;
    git(&["rebase", "FETCH_HEAD"])?;
    git(&["push", &remote, "HEAD:refs/heads/main"])?;
    Ok(())
}
