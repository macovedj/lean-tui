mod cast;
mod error;
mod publish;
mod render;
mod scene;
mod scenes;
mod stage;
mod terminal;

use std::{
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Parser, Subcommand};

use crate::{error::Result, scenes::SceneEntry};

#[derive(Parser)]
#[command(about = "Records the README GIFs of lean-tui next to Helix")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Record {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        font_dir: PathBuf,
        scenes: Vec<String>,
    },
    Publish {
        recordings: PathBuf,
        #[arg(long, default_value = "screenshots")]
        screenshots: PathBuf,
    },
}

fn selected_scenes(names: &[String]) -> Result<Vec<&'static SceneEntry>> {
    if names.is_empty() {
        Ok(scenes::SCENES.iter().collect())
    } else {
        names.iter().map(|name| scenes::find(name)).collect()
    }
}

fn record(out: &Path, font_dir: &Path, names: &[String]) -> Result<()> {
    fs::create_dir_all(out)?;
    selected_scenes(names)?.into_iter().try_for_each(|scene| {
        println!("Recording {}", scene.name);
        let cast_file = out.join(format!("{}.cast", scene.name));
        (scene.play)(&cast_file)?;
        render::render_gif(&cast_file, &out.join(format!("{}.gif", scene.name)), font_dir)?;
        Ok(fs::remove_file(cast_file)?)
    })
}

fn main() -> ExitCode {
    let outcome = match Cli::parse().command {
        Command::Record {
            out,
            font_dir,
            scenes,
        } => record(&out, &font_dir, &scenes),
        Command::Publish {
            recordings,
            screenshots,
        } => publish::publish(&recordings, &screenshots),
    };
    outcome.map_or_else(
        |error| {
            eprintln!("{error}");
            ExitCode::FAILURE
        },
        |()| ExitCode::SUCCESS,
    )
}
