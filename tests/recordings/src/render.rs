use std::{
    ffi::OsStr,
    path::Path,
    process::Command,
};

use crate::error::{RecordingError, Result};

fn run(tool: &'static str, args: &[&OsStr]) -> Result<()> {
    let status = Command::new(tool).args(args).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(RecordingError::ToolFailed { tool, status })
    }
}

pub fn render_gif(cast_file: &Path, gif_file: &Path, font_dir: &Path) -> Result<()> {
    run(
        "agg",
        &[
            OsStr::new("--font-dir"),
            font_dir.as_os_str(),
            OsStr::new("--font-family"),
            OsStr::new("JetBrains Mono"),
            OsStr::new("--last-frame-duration"),
            OsStr::new("0"),
            cast_file.as_os_str(),
            gif_file.as_os_str(),
        ],
    )?;
    run(
        "gifsicle",
        &[OsStr::new("--batch"), OsStr::new("-O3"), gif_file.as_os_str()],
    )
}
