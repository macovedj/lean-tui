use std::{fs, path::PathBuf};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};
use tokio::sync::mpsc::unbounded_channel;

use crate::{
    message::{find_click_target, AppMessage, ClickTarget},
    model::Model,
    protocol::ServerToClientMessage,
    update::update,
    view::view,
};

const TERMINAL_WIDTH: u16 = 140;
const TERMINAL_HEIGHT: u16 = 45;
const WORKER_PORT: u16 = 9742;

pub enum ReplayedMessages {
    All,
    First { count: usize },
}

pub enum ScenarioInput {
    Key { code: KeyCode },
    Click { column: u16, row: u16 },
}

pub const fn key(character: char) -> ScenarioInput {
    ScenarioInput::Key {
        code: KeyCode::Char(character),
    }
}

pub const fn click(column: u16, row: u16) -> ScenarioInput {
    ScenarioInput::Click { column, row }
}

pub fn fixture_messages(
    fixture_name: &str,
    replayed: &ReplayedMessages,
) -> Vec<ServerToClientMessage> {
    let path = PathBuf::from(env!("LEAN_TUI_DEMOS_DIR"))
        .join(fixture_name)
        .join("replay.jsonl");
    let messages = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
        .lines()
        .map(|line| serde_json::from_str(line).expect("fixture line parses"))
        .collect::<Vec<_>>();
    match *replayed {
        ReplayedMessages::All => messages,
        ReplayedMessages::First { count } => messages.into_iter().take(count).collect(),
    }
}

fn input_message(input: &ScenarioInput, click_targets: &[ClickTarget]) -> Option<AppMessage> {
    match *input {
        ScenarioInput::Key { code } => Some(AppMessage::KeyPressed {
            key: KeyEvent::new(code, KeyModifiers::NONE),
        }),
        ScenarioInput::Click { column, row } => find_click_target(click_targets, column, row),
    }
}

pub fn press_key(model: &mut Model, character: char) {
    if let Some(message) = input_message(&key(character), &[]) {
        update(model, message);
    }
}

pub fn connected_model() -> Model {
    let mut model = Model::new(WORKER_PORT..=WORKER_PORT + 10);
    let (commands, _pending_commands) = unbounded_channel();
    update(
        &mut model,
        AppMessage::WorkerConnected {
            port: WORKER_PORT,
            commands,
        },
    );
    model
}

pub fn send_from_worker(model: &mut Model, message: ServerToClientMessage) {
    update(
        model,
        AppMessage::WorkerSent {
            port: WORKER_PORT,
            message,
        },
    );
}

pub fn render_scenario(
    fixture_name: &str,
    replayed: &ReplayedMessages,
    inputs: &[ScenarioInput],
) -> String {
    let mut terminal =
        Terminal::new(TestBackend::new(TERMINAL_WIDTH, TERMINAL_HEIGHT)).expect("test terminal");
    let messages = fixture_messages(fixture_name, replayed);
    let mut model = if messages.is_empty() {
        Model::new(WORKER_PORT..=WORKER_PORT + 10)
    } else {
        connected_model()
    };
    let mut click_targets = Vec::new();
    let mut draw = |model: &Model, click_targets: &mut Vec<ClickTarget>| {
        terminal
            .draw(|frame| *click_targets = view(model, frame))
            .expect("draw");
    };

    draw(&model, &mut click_targets);
    for message in messages {
        send_from_worker(&mut model, message);
        draw(&model, &mut click_targets);
    }
    for input in inputs {
        if let Some(message) = input_message(input, &click_targets) {
            update(&mut model, message);
        }
        draw(&model, &mut click_targets);
    }
    format!("{:?}", terminal.backend().buffer())
}
