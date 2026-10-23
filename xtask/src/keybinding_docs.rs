use std::{fs, path::Path};

use lean_tui::keybinding_reference_markdown;

use crate::command_runner::TaskError;

pub fn write_keybinding_docs(docs_path: &Path, check_only: bool) -> Result<(), TaskError> {
    let generated_markdown = keybinding_reference_markdown();
    if !check_only {
        return Ok(fs::write(docs_path, generated_markdown)?);
    }
    if fs::read_to_string(docs_path)? == generated_markdown {
        Ok(())
    } else {
        Err(TaskError::OutdatedFile {
            path: docs_path.to_path_buf(),
            regenerate_command: "cargo xtask keybinding-docs",
        })
    }
}
