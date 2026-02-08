//! Centralized theme and styling constants.

use ratatui::style::{Color, Modifier, Style};

pub struct Blue;

impl Blue {
    pub const MUTED: Color = Color::Rgb(100, 140, 180); // Pane titles
    pub const BRIGHT: Color = Color::Rgb(100, 180, 255); // Names, active elements
    pub const BG: Color = Color::Rgb(30, 40, 55); // Background tint
}

pub struct Orange;

impl Orange {
    pub const MUTED: Color = Color::Rgb(200, 140, 100); // Pane titles
    pub const BRIGHT: Color = Color::Rgb(255, 180, 100); // Goals, active elements
}

pub struct Yellow;

impl Yellow {
    pub const MUTED: Color = Color::Rgb(180, 160, 100); // Tactic titles, dependencies
    pub const BRIGHT: Color = Color::Rgb(220, 180, 80); // Changed elements
    pub const VIVID: Color = Color::Rgb(255, 200, 80); // Incomplete nodes, warnings
}

pub struct Green;

impl Green {
    pub const BRIGHT: Color = Color::Rgb(100, 200, 100); // Added elements
}

pub struct Red;

impl Red {
    pub const MUTED: Color = Color::Rgb(180, 100, 100); // Error borders
    pub const BRIGHT: Color = Color::Rgb(220, 100, 100); // Removed elements
}

pub struct Purple;

impl Purple {
    pub const BRIGHT: Color = Color::Rgb(160, 120, 200);
}

pub struct Gray;

impl Gray {
    pub const DIM: Color = Color::DarkGray;
}

pub struct Theme;

impl Theme {
    // === General Styles ===
    pub const KEYBINDING: Color = Gray::DIM;

    // === Frame & Border Colors ===
    pub const PANE_BORDER_FOCUSED: Color = Gray::DIM;
    pub const FRAME_BORDER: Color = Color::Reset; // Terminal default for outer frame
    pub const SELECTION_BG: Color = Color::Rgb(50, 50, 80);

    // === Assumption/Input/Given Pane Colors ===
    pub const TITLE_ASSUMPTION: Color = Blue::MUTED;
    pub const HYP_NAME: Color = Blue::BRIGHT;

    // === Goal/Output Pane Colors ===
    pub const TITLE_GOAL: Color = Orange::MUTED;
    pub const GOAL: Color = Orange::BRIGHT;

    // === Tactic/Step Pane Colors ===
    pub const TITLE_TACTIC: Color = Yellow::MUTED;

    // === Diff Colors ===
    pub const ADDED: Color = Green::BRIGHT;
    pub const REMOVED: Color = Red::BRIGHT;
    pub const CHANGED: Color = Yellow::BRIGHT;
    pub const UNCHANGED: Color = Color::White;
    // === Graph Node Border Colors ===
    pub const INACTIVE_NODE_BORDER: Color = Gray::DIM;
    pub const CURRENT_NODE_BORDER: Color = Color::White;
    pub const COMPLETED_NODE_BORDER: Color = Purple::BRIGHT;
    pub const INCOMPLETE_NODE_BORDER: Color = Yellow::VIVID;
    pub const COMPLETED_GOAL_FG: Color = Purple::BRIGHT;

    // === Hypothesis Background Colors (by type) ===
    pub const DATA_HYP_BG: Color = Blue::BG;
    pub const DATA_HYP_FG: Color = Color::Rgb(130, 170, 220);

    // === Dependency Styling ===
    pub const DEPENDENCY: Style = Style::new().fg(Blue::MUTED).add_modifier(Modifier::DIM);

    // === Effect Flow Colors ===
    pub const EFFECT_BIND: Color = Color::Cyan;
    pub const EFFECT_STATE_READ: Color = Green::BRIGHT;
    pub const EFFECT_STATE_WRITE: Color = Color::Rgb(240, 180, 80); // Golden
    pub const EFFECT_STATE_MODIFY: Color = Color::Magenta;
    pub const EFFECT_RETURN: Color = Color::Rgb(100, 140, 200); // Blue
    pub const EFFECT_THROW: Color = Red::BRIGHT;
    pub const EFFECT_CATCH: Color = Color::Rgb(255, 140, 140); // Light red
    pub const EFFECT_PURE: Color = Color::White;

    // === Edge Colors (Graph rendering) ===
    pub const EDGE_DATA_FLOW: Color = Color::Rgb(120, 80, 140); // Dimmed purple (expand)
    pub const EDGE_CONTROL_FLOW: Color = Color::Rgb(160, 160, 160); // Silver
    pub const EDGE_EFFECT_FLOW: Color = Color::Rgb(180, 100, 200); // Purple
    pub const EDGE_ERROR_PROPAGATION: Color = Color::Rgb(255, 130, 80); // Orange
    pub const EDGE_EARLY_RETURN: Color = Color::Rgb(255, 190, 90); // Gold
    pub const EDGE_CONTINUATION: Color = Color::Rgb(80, 100, 140); // Dimmed blue (then)

    // === Data Flow Node Colors ===
    pub const DATAFLOW_BINDING: Color = Color::Cyan;
    pub const DATAFLOW_LAMBDA: Color = Color::Magenta;
    pub const DATAFLOW_BRANCH: Color = Color::Rgb(255, 200, 100); // Amber
    pub const DATAFLOW_CONDITION: Color = Color::Rgb(100, 140, 200); // Blue
}
