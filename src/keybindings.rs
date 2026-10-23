use crossterm::event::KeyCode;

use crate::{
    message::{GlobalAction, ModeAction, WorkerSelectorAction},
    navigation::Direction,
};

#[derive(Clone, Copy)]
pub struct KeyAction<A: 'static> {
    pub key: KeyCode,
    pub action: A,
}

pub struct KeyBinding<A: 'static> {
    pub label: &'static str,
    pub help_description: &'static str,
    pub status_description: Option<&'static str>,
    pub key_actions: &'static [KeyAction<A>],
}

pub fn find_key_action<A: Copy>(bindings: &[KeyBinding<A>], key: KeyCode) -> Option<A> {
    bindings
        .iter()
        .flat_map(|binding| binding.key_actions)
        .find(|key_action| key_action.key == key)
        .map(|key_action| key_action.action)
}

const fn global(key: char, action: GlobalAction) -> KeyAction<GlobalAction> {
    KeyAction {
        key: KeyCode::Char(key),
        action,
    }
}

pub const GLOBAL_KEYBINDINGS: &[KeyBinding<GlobalAction>] = &[
    KeyBinding {
        label: "?",
        help_description: "toggle help",
        status_description: Some("help"),
        key_actions: &[global('?', GlobalAction::ToggleHelp)],
    },
    KeyBinding {
        label: "[/]",
        help_description: "cycle display mode",
        status_description: Some("style"),
        key_actions: &[
            global(']', GlobalAction::NextDisplayMode),
            global('[', GlobalAction::PreviousDisplayMode),
        ],
    },
    KeyBinding {
        label: "i/o",
        help_description: "toggle input/output panes",
        status_description: Some("in/out"),
        key_actions: &[
            global('i', GlobalAction::ToggleInputPane),
            global('o', GlobalAction::ToggleOutputPane),
        ],
    },
    KeyBinding {
        label: "m",
        help_description: "cycle timing metrics",
        status_description: Some("metrics"),
        key_actions: &[global('m', GlobalAction::CycleTimingDisplay)],
    },
    KeyBinding {
        label: "g",
        help_description: "goto origin",
        status_description: Some("goto"),
        key_actions: &[
            global('g', GlobalAction::GotoSelection),
            KeyAction {
                key: KeyCode::Enter,
                action: GlobalAction::GotoSelection,
            },
        ],
    },
    KeyBinding {
        label: "y",
        help_description: "copy to clipboard",
        status_description: Some("copy"),
        key_actions: &[global('y', GlobalAction::CopySelection)],
    },
    KeyBinding {
        label: "f",
        help_description: "select file",
        status_description: None,
        key_actions: &[global('f', GlobalAction::OpenWorkerSelector)],
    },
    KeyBinding {
        label: "F",
        help_description: "toggle follow mode",
        status_description: None,
        key_actions: &[global('F', GlobalAction::ToggleFollowMode)],
    },
    KeyBinding {
        label: "L",
        help_description: "toggle legend",
        status_description: None,
        key_actions: &[global('L', GlobalAction::ToggleLegend)],
    },
    KeyBinding {
        label: "Esc",
        help_description: "close popup",
        status_description: None,
        key_actions: &[KeyAction {
            key: KeyCode::Esc,
            action: GlobalAction::CloseOverlay,
        }],
    },
    KeyBinding {
        label: "q",
        help_description: "quit",
        status_description: Some("quit"),
        key_actions: &[global('q', GlobalAction::Quit)],
    },
];

const fn move_on(key: KeyCode, direction: Direction) -> KeyAction<ModeAction> {
    KeyAction {
        key,
        action: ModeAction::Move { direction },
    }
}

const VERTICAL_MOVES: [KeyAction<ModeAction>; 4] = [
    move_on(KeyCode::Char('j'), Direction::Down),
    move_on(KeyCode::Down, Direction::Down),
    move_on(KeyCode::Char('k'), Direction::Up),
    move_on(KeyCode::Up, Direction::Up),
];

const HORIZONTAL_MOVES: [KeyAction<ModeAction>; 4] = [
    move_on(KeyCode::Char('h'), Direction::Left),
    move_on(KeyCode::Left, Direction::Left),
    move_on(KeyCode::Char('l'), Direction::Right),
    move_on(KeyCode::Right, Direction::Right),
];

const ALL_MOVES: [KeyAction<ModeAction>; 8] = [
    VERTICAL_MOVES[0],
    VERTICAL_MOVES[1],
    VERTICAL_MOVES[2],
    VERTICAL_MOVES[3],
    HORIZONTAL_MOVES[0],
    HORIZONTAL_MOVES[1],
    HORIZONTAL_MOVES[2],
    HORIZONTAL_MOVES[3],
];

const fn mode(key: char, action: ModeAction) -> KeyAction<ModeAction> {
    KeyAction {
        key: KeyCode::Char(key),
        action,
    }
}

const GRAPH_NAVIGATION: KeyBinding<ModeAction> = KeyBinding {
    label: "hjkl",
    help_description: "navigate",
    status_description: Some("nav"),
    key_actions: &ALL_MOVES,
};

const FLIP_ORIENTATION: KeyBinding<ModeAction> = KeyBinding {
    label: "t",
    help_description: "flip tree orientation",
    status_description: Some("flip"),
    key_actions: &[mode('t', ModeAction::FlipOrientation)],
};

pub const SEMANTIC_TABLEAU_KEYBINDINGS: &[KeyBinding<ModeAction>] =
    &[GRAPH_NAVIGATION, FLIP_ORIENTATION];

pub const DATA_FLOW_KEYBINDINGS: &[KeyBinding<ModeAction>] = &[GRAPH_NAVIGATION, FLIP_ORIENTATION];

pub const EFFECT_FLOW_KEYBINDINGS: &[KeyBinding<ModeAction>] = &[
    GRAPH_NAVIGATION,
    KeyBinding {
        label: "e",
        help_description: "expand pure steps",
        status_description: Some("expand"),
        key_actions: &[mode('e', ModeAction::ToggleExpanded)],
    },
];

const LIST_SELECTION: KeyBinding<ModeAction> = KeyBinding {
    label: "jk",
    help_description: "select hypothesis or goal",
    status_description: Some("select"),
    key_actions: &VERTICAL_MOVES,
};

pub const COMPACT_TREE_KEYBINDINGS: &[KeyBinding<ModeAction>] = &[
    LIST_SELECTION,
    KeyBinding {
        label: "hl",
        help_description: "scroll steps",
        status_description: Some("scroll"),
        key_actions: &HORIZONTAL_MOVES,
    },
];

const fn selector(key: KeyCode, action: WorkerSelectorAction) -> KeyAction<WorkerSelectorAction> {
    KeyAction { key, action }
}

pub const WORKER_SELECTOR_KEYBINDINGS: &[KeyBinding<WorkerSelectorAction>] = &[
    KeyBinding {
        label: "↑/↓",
        help_description: "select",
        status_description: None,
        key_actions: &[
            selector(KeyCode::Up, WorkerSelectorAction::HighlightPrevious),
            selector(KeyCode::Char('k'), WorkerSelectorAction::HighlightPrevious),
            selector(KeyCode::Down, WorkerSelectorAction::HighlightNext),
            selector(KeyCode::Char('j'), WorkerSelectorAction::HighlightNext),
        ],
    },
    KeyBinding {
        label: "Enter",
        help_description: "connect",
        status_description: None,
        key_actions: &[selector(KeyCode::Enter, WorkerSelectorAction::Connect)],
    },
    KeyBinding {
        label: "r",
        help_description: "rescan",
        status_description: None,
        key_actions: &[selector(KeyCode::Char('r'), WorkerSelectorAction::Rescan)],
    },
    KeyBinding {
        label: "Esc",
        help_description: "cancel",
        status_description: None,
        key_actions: &[selector(KeyCode::Esc, WorkerSelectorAction::Cancel)],
    },
];

pub const BEFORE_AFTER_KEYBINDINGS: &[KeyBinding<ModeAction>] = &[
    LIST_SELECTION,
    KeyBinding {
        label: "p",
        help_description: "toggle previous step",
        status_description: Some("prev"),
        key_actions: &[mode('p', ModeAction::TogglePreviousPane)],
    },
    KeyBinding {
        label: "n",
        help_description: "toggle next step",
        status_description: Some("next"),
        key_actions: &[mode('n', ModeAction::ToggleNextPane)],
    },
];
