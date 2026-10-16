mod data_flow;
mod effect_flow;
mod semantic_tableau;
mod workflow;

use std::path::Path;

use crate::error::{RecordingError, Result};

pub struct SceneEntry {
    pub name: &'static str,
    pub play: fn(&Path) -> Result<()>,
}

pub const SCENES: [SceneEntry; 4] = [
    SceneEntry {
        name: "semantic_tableau",
        play: semantic_tableau::play,
    },
    SceneEntry {
        name: "data_flow",
        play: data_flow::play,
    },
    SceneEntry {
        name: "effect_flow",
        play: effect_flow::play,
    },
    SceneEntry {
        name: "workflow",
        play: workflow::play,
    },
];

pub fn find(name: &str) -> Result<&'static SceneEntry> {
    SCENES
        .iter()
        .find(|scene| scene.name == name)
        .ok_or_else(|| RecordingError::UnknownScene(name.to_owned()))
}
