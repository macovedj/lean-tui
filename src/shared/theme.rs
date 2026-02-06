//! Centralized theme and styling constants.

use ratatui::style::{Color, Modifier, Style};

/// Theme constants for consistent styling across components.
pub struct Theme;

impl Theme {
    pub const DIM: Style = Style::new().fg(Color::DarkGray);
    pub const DEPENDENCY: Style = Style::new().fg(Color::Yellow).add_modifier(Modifier::DIM);

    pub const BORDER: Color = Color::DarkGray;
    pub const BORDER_FOCUSED: Color = Color::Cyan;
    pub const TITLE_HYPOTHESIS: Color = Color::Blue;
    pub const TITLE_GOAL: Color = Color::Cyan;
    pub const SELECTION_BG: Color = Color::Rgb(50, 50, 80);

    pub const HYP_NAME: Color = Color::Blue;
    pub const HYP_TYPE: Color = Color::White;
    pub const GOAL_TYPE: Color = Color::Cyan;

    pub const PROOF_HYP_BG: Color = Color::Rgb(30, 45, 30);
    pub const PROOF_HYP_FG: Color = Color::Rgb(150, 200, 150);
    pub const DATA_HYP_BG: Color = Color::Rgb(50, 45, 25);
    pub const DATA_HYP_FG: Color = Color::Rgb(200, 180, 120);

    pub const TACTIC_BORDER: Color = Color::DarkGray;
    pub const CURRENT_NODE_BORDER: Color = Color::Cyan;
    pub const INCOMPLETE_NODE_BORDER: Color = Color::Yellow;
    pub const COMPLETED_NODE_BORDER: Color = Color::Green;
    pub const GOAL_FG: Color = Color::Rgb(200, 140, 140);
    pub const COMPLETED_GOAL_FG: Color = Color::Green;
    pub const NODE_BG: Color = Color::Rgb(30, 30, 30);

    pub const EFFECT_BIND: Color = Color::Cyan;
    pub const EFFECT_STATE_READ: Color = Color::Green;
    pub const EFFECT_STATE_WRITE: Color = Color::Yellow;
    pub const EFFECT_STATE_MODIFY: Color = Color::Magenta;
    pub const EFFECT_RETURN: Color = Color::Blue;
    pub const EFFECT_THROW: Color = Color::Red;
    pub const EFFECT_CATCH: Color = Color::LightRed;
    pub const EFFECT_PURE: Color = Color::White;

    pub const EDGE_DATA_FLOW: Color = Color::Cyan;
    pub const EDGE_CONTROL_FLOW: Color = Color::White;
    pub const EDGE_EFFECT_FLOW: Color = Color::Magenta;
    pub const EDGE_ERROR_PROPAGATION: Color = Color::Red;
    pub const EDGE_EARLY_RETURN: Color = Color::Yellow;
    pub const EDGE_CONTINUATION: Color = Color::DarkGray;

    pub const DATAFLOW_BINDING: Color = Color::Cyan;
    pub const DATAFLOW_LAMBDA: Color = Color::Magenta;
    pub const DATAFLOW_BRANCH: Color = Color::Yellow;
    pub const DATAFLOW_CONDITION: Color = Color::Blue;
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
