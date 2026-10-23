use crossterm::event::KeyEvent;
use ratatui::layout::{Position, Rect};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    navigation::Direction,
    protocol::{ClientToServerCommand, ServerToClientMessage},
    selection::Selection,
    tcp_client::WorkerInfo,
};

#[derive(Clone)]
pub enum AppMessage {
    KeyPressed {
        key: KeyEvent,
    },
    WorkerConnected {
        port: u16,
        commands: UnboundedSender<ClientToServerCommand>,
    },
    WorkerSent {
        port: u16,
        message: ServerToClientMessage,
    },
    WorkerSentInvalid {
        port: u16,
        error: String,
    },
    WorkerDisconnected {
        port: u16,
    },
    WorkersDiscovered {
        workers: Vec<WorkerInfo>,
    },
    Select {
        selection: Selection,
    },
    ShowError {
        message: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlobalAction {
    Quit,
    ToggleHelp,
    ToggleLegend,
    ToggleInputPane,
    ToggleOutputPane,
    OpenWorkerSelector,
    ToggleFollowMode,
    NextDisplayMode,
    PreviousDisplayMode,
    GotoSelection,
    CopySelection,
    CycleTimingDisplay,
    CloseOverlay,
}

#[derive(Debug, Clone, Copy)]
pub enum ModeAction {
    Move { direction: Direction },
    FlipOrientation,
    ToggleExpanded,
    TogglePreviousPane,
    ToggleNextPane,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerSelectorAction {
    HighlightPrevious,
    HighlightNext,
    Connect,
    Rescan,
    Cancel,
}

#[derive(Clone)]
pub struct ClickTarget {
    pub area: Rect,
    pub message: AppMessage,
}

pub fn find_click_target(
    click_targets: &[ClickTarget],
    column: u16,
    row: u16,
) -> Option<AppMessage> {
    click_targets
        .iter()
        .rev()
        .find(|target| target.area.contains(Position::new(column, row)))
        .map(|target| target.message.clone())
}

pub fn selection_click_target(area: Rect, selection: Selection) -> ClickTarget {
    ClickTarget {
        area,
        message: AppMessage::Select { selection },
    }
}
