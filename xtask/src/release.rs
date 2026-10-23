use std::{fs, path::Path, process::Command};

use clap::ValueEnum;

use crate::{
    command_runner::{command_stdout, required_variable, run_command, TaskError},
    directory_files::{file_names, files_with_suffix, regular_files},
};

const TEA_LOGIN: &str = "codeberg";

#[derive(Clone, Copy, ValueEnum)]
pub enum ReleasePlzCommand {
    Release,
    ReleasePr,
}

impl ReleasePlzCommand {
    const fn subcommand_name(self) -> &'static str {
        match self {
            Self::Release => "release",
            Self::ReleasePr => "release-pr",
        }
    }
}

pub fn upload_release_assets(assets_dir: &Path) -> Result<(), TaskError> {
    let tarballs = files_with_suffix(assets_dir, ".tar.gz")?;
    let checksums = command_stdout(
        Command::new("sha256sum")
            .args(file_names(&tarballs))
            .current_dir(assets_dir),
    )?;
    fs::write(assets_dir.join("sha256sum.txt"), checksums)?;
    let forge_url = required_variable("CI_FORGE_URL")?;
    let token = required_variable("CODEBERG_TOKEN")?;
    let repository = required_variable("CI_REPO")?;
    let release_tag = required_variable("CI_COMMIT_TAG")?;
    run_command(Command::new("tea").args([
        "login", "add", "--name", TEA_LOGIN, "--url", &forge_url, "--token", &token,
    ]))?;
    let release_assets = regular_files(assets_dir)?;
    run_command(
        Command::new("tea")
            .args([
                "releases",
                "assets",
                "create",
                "--login",
                TEA_LOGIN,
                "--repo",
                &repository,
                &release_tag,
            ])
            .args(file_names(&release_assets))
            .current_dir(assets_dir),
    )
}

pub fn run_release_plz(release_plz_command: ReleasePlzCommand) -> Result<(), TaskError> {
    let token = required_variable("CODEBERG_TOKEN")?;
    run_command(Command::new("release-plz").args([
        release_plz_command.subcommand_name(),
        "--forge",
        "gitea",
        "--git-token",
        &token,
    ]))
}
