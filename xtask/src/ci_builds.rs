use std::{
    env,
    process::{Command, Stdio},
};

use crate::command_runner::{run_command, wait_for_child, TaskError};

const CACHIX_CACHE: &str = "wvhulle";

fn cachix_token_present() -> bool {
    env::var("CACHIX_AUTH_TOKEN").is_ok_and(|token| !token.is_empty())
}

pub fn cached_build(build_args: &[String]) -> Result<(), TaskError> {
    if !cachix_token_present() {
        return run_command(
            Command::new("nix")
                .args(["build", "--print-build-logs"])
                .args(build_args),
        );
    }
    let mut watched_build = Command::new("cachix");
    watched_build
        .args([
            "watch-exec",
            CACHIX_CACHE,
            "--",
            "nix",
            "build",
            "--print-build-logs",
            "--print-out-paths",
        ])
        .args(build_args)
        .stdout(Stdio::piped());
    let mut watched_build_child = watched_build.spawn()?;
    let out_paths = watched_build_child
        .stdout
        .take()
        .map_or_else(Stdio::null, Stdio::from);
    let push_result = run_command(
        Command::new("cachix")
            .args(["push", CACHIX_CACHE])
            .stdin(out_paths),
    );
    wait_for_child(&watched_build, watched_build_child)?;
    push_result
}

pub fn check_commits(revision_range: &[String]) -> Result<(), TaskError> {
    run_command(Command::new("convco").arg("check").args(revision_range))
}
