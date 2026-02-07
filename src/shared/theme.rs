//! Centralized theme and styling constants.

use ratatui::style::{Color, Modifier, Style};

// Color family structs provide organized color palettes.
// Some shades may be unused but are kept for consistency and future use.

/// Blue color family - for assumptions, inputs, hypotheses
#[allow(dead_code)]
pub struct Blue;

#[allow(dead_code)]
impl Blue {
    pub const MUTED: Color = Color::Rgb(100, 140, 180); // Pane titles
    pub const BRIGHT: Color = Color::Rgb(100, 180, 255); // Names, active elements
    pub const BG: Color = Color::Rgb(30, 40, 55); // Background tint
}

/// Orange color family - for goals, outputs
#[allow(dead_code)]
pub struct Orange;

#[allow(dead_code)]
impl Orange {
    pub const MUTED: Color = Color::Rgb(200, 140, 100); // Pane titles
    pub const BRIGHT: Color = Color::Rgb(255, 180, 100); // Goals, active elements
    pub const BG: Color = Color::Rgb(50, 35, 25); // Background tint for goals
}

/// Yellow color family - for tactics, dependencies, incomplete states
#[allow(dead_code)]
pub struct Yellow;

#[allow(dead_code)]
impl Yellow {
    pub const MUTED: Color = Color::Rgb(180, 160, 100); // Tactic titles, dependencies
    pub const BRIGHT: Color = Color::Rgb(220, 180, 80); // Changed elements
    pub const VIVID: Color = Color::Rgb(255, 200, 80); // Incomplete nodes, warnings
}

/// Green color family - for added/new elements, completion
#[allow(dead_code)]
pub struct Green;

#[allow(dead_code)]
impl Green {
    pub const MUTED: Color = Color::Rgb(100, 160, 100); // Success states
    pub const BRIGHT: Color = Color::Rgb(100, 200, 100); // Added elements
    pub const BG: Color = Color::Rgb(30, 45, 30); // Background tint
    pub const FG: Color = Color::Rgb(150, 200, 150); // Foreground for proof terms
}

/// Red color family - for removed elements, errors
#[allow(dead_code)]
pub struct Red;

#[allow(dead_code)]
impl Red {
    pub const MUTED: Color = Color::Rgb(180, 100, 100); // Error borders
    pub const BRIGHT: Color = Color::Rgb(220, 100, 100); // Removed elements
}

/// Purple color family - for completed/closed nodes
#[allow(dead_code)]
pub struct Purple;

#[allow(dead_code)]
impl Purple {
    pub const MUTED: Color = Color::Rgb(140, 100, 160); // Completed borders
    pub const BRIGHT: Color = Color::Rgb(160, 120, 200); // Completed goals/nodes
}

/// Gray color family - for inactive/dim elements
#[allow(dead_code)]
pub struct Gray;

#[allow(dead_code)]
impl Gray {
    pub const DIM: Color = Color::DarkGray; // Borders, keybindings
    pub const MUTED: Color = Color::Rgb(120, 120, 120); // Inactive text
    pub const BRIGHT: Color = Color::Rgb(180, 180, 180); // Muted white
}

/// Theme constants for consistent styling across components.
pub struct Theme;

impl Theme {
    // === General Styles ===
    pub const DIM: Style = Style::new().fg(Gray::DIM);
    pub const KEYBINDING: Color = Gray::DIM;

    // === Frame & Border Colors ===
    pub const BORDER: Color = Gray::DIM;
    pub const BORDER_FOCUSED: Color = Blue::MUTED;
    pub const FRAME_BORDER: Color = Color::Reset; // Terminal default for outer frame
    pub const SELECTION_BG: Color = Color::Rgb(50, 50, 80);

    // === Assumption/Input/Given Pane Colors ===
    pub const TITLE_ASSUMPTION: Color = Blue::MUTED;
    pub const HYP_NAME: Color = Blue::BRIGHT;
    pub const HYP_TYPE: Color = Color::White;

    // === Goal/Output Pane Colors ===
    pub const TITLE_GOAL: Color = Orange::MUTED;
    pub const GOAL_FG: Color = Orange::BRIGHT;

    // === Tactic/Step Pane Colors ===
    pub const TITLE_TACTIC: Color = Yellow::MUTED;

    // === Diff Colors ===
    pub const ADDED: Color = Green::BRIGHT;
    pub const REMOVED: Color = Red::BRIGHT;
    pub const CHANGED: Color = Yellow::BRIGHT;

    // === Graph Node Border Colors ===
    pub const TACTIC_BORDER: Color = Gray::DIM;
    pub const CURRENT_NODE_BORDER: Color = Color::White; // Bright white for active node
    pub const COMPLETED_NODE_BORDER: Color = Purple::BRIGHT;
    pub const INCOMPLETE_NODE_BORDER: Color = Yellow::VIVID; // Bright yellow for open goals
    pub const COMPLETED_GOAL_FG: Color = Purple::BRIGHT;

    // === Hypothesis Background Colors (by type) ===
    pub const PROOF_HYP_BG: Color = Green::BG;
    pub const PROOF_HYP_FG: Color = Green::FG;
    pub const DATA_HYP_BG: Color = Blue::BG;
    pub const DATA_HYP_FG: Color = Color::Rgb(130, 170, 220); // Blue fg for data

    // === Dependency Styling ===
    pub const DEPENDENCY_FG: Color = Yellow::MUTED;
    pub const DEPENDENCY: Style = Style::new().fg(Yellow::MUTED).add_modifier(Modifier::DIM);

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
    pub const EDGE_DATA_FLOW: Color = Color::Rgb(100, 180, 255); // Soft blue
    pub const EDGE_CONTROL_FLOW: Color = Color::Rgb(160, 160, 160); // Silver
    pub const EDGE_EFFECT_FLOW: Color = Color::Rgb(180, 100, 200); // Purple
    pub const EDGE_ERROR_PROPAGATION: Color = Color::Rgb(255, 130, 80); // Orange
    pub const EDGE_EARLY_RETURN: Color = Color::Rgb(255, 190, 90); // Gold
    pub const EDGE_CONTINUATION: Color = Color::Rgb(100, 100, 100); // Dim gray

    // === Data Flow Node Colors ===
    pub const DATAFLOW_BINDING: Color = Color::Cyan;
    pub const DATAFLOW_LAMBDA: Color = Color::Magenta;
    pub const DATAFLOW_BRANCH: Color = Color::Rgb(255, 200, 100); // Amber
    pub const DATAFLOW_CONDITION: Color = Color::Rgb(100, 140, 200); // Blue

    // === Legacy aliases for backwards compatibility ===
    pub const TITLE_HYPOTHESIS: Color = Self::TITLE_ASSUMPTION;
    pub const GOAL_TYPE: Color = Self::GOAL_FG;
}

/// Which pane is currently focused for keyboard navigation.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    #[default]
    Sidebar,
    Hypotheses,
    Goals,
}

impl FocusedPane {
    /// Cycle to the next pane (Sidebar → Hypotheses → Goals → Sidebar).
    pub const fn next(self) -> Self {
        match self {
            Self::Sidebar => Self::Hypotheses,
            Self::Hypotheses => Self::Goals,
            Self::Goals => Self::Sidebar,
        }
    }

    /// Cycle to the previous pane (reverse of next).
    pub const fn prev(self) -> Self {
        match self {
            Self::Sidebar => Self::Goals,
            Self::Hypotheses => Self::Sidebar,
            Self::Goals => Self::Hypotheses,
        }
    }
}
