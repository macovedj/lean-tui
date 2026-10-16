use std::{path::Path, thread, time::Duration};

use crate::{
    cast::CastWriter,
    error::Result,
    stage::{Focus, Stage, StageSize},
    terminal::Key,
};

const STAGE_SIZE: StageSize = StageSize { rows: 40, cols: 160 };
const VIEW_PLACEHOLDERS: [&str; 2] = ["Connecting to", "Move cursor to a tactic"];
const KEY_HOLD: Duration = Duration::from_millis(400);
const STEP_HOLD: Duration = Duration::from_millis(900);
const RESULT_HOLD: Duration = Duration::from_millis(1800);

fn view_is_connected(stage: &Stage) -> bool {
    let view = stage.view_text();
    VIEW_PLACEHOLDERS.iter().all(|placeholder| !view.contains(placeholder))
}

fn view_follows_line(stage: &Stage, line: u32) -> bool {
    stage
        .view_text()
        .lines()
        .any(|row| row.contains(&format!(" {line}:")) && row.contains("(hover)"))
}

fn view_shows_mode(stage: &Stage, mode: &str) -> bool {
    let title = stage.view_title();
    let without_corner: String = {
        let trimmed = title.trim_end();
        let mut characters = trimmed.chars();
        characters.next_back();
        characters.collect()
    };
    without_corner.ends_with(&format!(" {mode} "))
}

fn goto_line(stage: &Stage, line: u32) -> Result<()> {
    stage.send(Focus::Editor, &Key::Text(&format!(":{line}")))?;
    stage.send(Focus::Editor, &Key::Enter)?;
    stage.send(Focus::Editor, &Key::Text("gs"))
}

fn hover(stage: &Stage) -> Result<()> {
    stage.send(Focus::Editor, &Key::Text(" k"))?;
    thread::sleep(Duration::from_secs(1));
    stage.send(Focus::Editor, &Key::Escape)
}

pub struct Rehearsal {
    stage: Stage,
}

impl Rehearsal {
    pub fn open(lean_file: &str, warmup_line: u32) -> Result<Self> {
        let stage = Stage::open(lean_file, STAGE_SIZE)?;
        stage.retry_until(
            "the Lean file in the editor",
            |stage| stage.editor_text().contains("import LeanPrism"),
            |_| {
                thread::sleep(Duration::from_millis(200));
                Ok(())
            },
        )?;
        goto_line(&stage, warmup_line)?;
        stage.retry_until("a connection to lean-prism", view_is_connected, hover)?;
        stage.settle()?;
        goto_line(&stage, warmup_line)?;
        stage.retry_until(
            &format!("the view to follow line {warmup_line}"),
            |stage| view_follows_line(stage, warmup_line),
            hover,
        )?;
        Ok(Self { stage })
    }

    pub fn switch_view_to(self, mode: &str) -> Result<Self> {
        self.stage.retry_until(
            mode,
            |stage| view_shows_mode(stage, mode),
            |stage| {
                stage.send(Focus::View, &Key::Text("]"))?;
                stage.settle()
            },
        )?;
        Ok(self)
    }

    pub fn start_take(self, cast_file: &Path) -> Result<Take> {
        self.stage.settle()?;
        self.stage.send(Focus::Editor, &Key::Escape)?;
        self.stage.settle()?;
        let mut cast = CastWriter::create(cast_file, self.stage.size())?;
        cast.snapshot(self.stage.frame(), STEP_HOLD)?;
        Ok(Take {
            stage: self.stage,
            cast,
        })
    }
}

pub struct Take {
    stage: Stage,
    cast: CastWriter,
}

impl Take {
    fn capture(&mut self, hold: Duration) -> Result<()> {
        self.stage.settle()?;
        self.cast.snapshot(self.stage.frame(), hold)
    }

    pub fn inspect_line(&mut self, line: u32) -> Result<()> {
        goto_line(&self.stage, line)?;
        self.capture(STEP_HOLD)?;
        self.stage.send(Focus::Editor, &Key::Text(" k"))?;
        self.capture(STEP_HOLD)?;
        self.stage.send(Focus::Editor, &Key::Escape)?;
        self.capture(RESULT_HOLD)
    }

    pub fn focus(&mut self, focus: Focus) -> Result<()> {
        self.stage.focus(focus);
        self.capture(KEY_HOLD)
    }

    pub fn press(&mut self, keys: &[&str]) -> Result<()> {
        keys.iter().try_for_each(|key| {
            self.stage.send_to_focused(&Key::Text(key))?;
            self.capture(KEY_HOLD)
        })
    }

    pub fn escape(&mut self) -> Result<()> {
        self.stage.send_to_focused(&Key::Escape)?;
        self.capture(KEY_HOLD)
    }

    pub fn hold(&mut self, duration: Duration) -> Result<()> {
        self.capture(duration)
    }

    pub fn finish(self) -> Result<()> {
        self.cast.finish()
    }
}
