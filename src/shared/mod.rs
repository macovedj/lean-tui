pub mod aligned_table;
pub mod assumption_pane;
pub mod diff_text;
pub mod error_banner;
pub mod errors_section;
pub mod goals_pane;
pub mod interactive;
pub mod legend;
pub mod navigation;
pub mod render_helpers;
pub mod selection;
pub mod theme;

pub use assumption_pane::{AssumptionPane, AssumptionPaneState};
pub use crossterm::event::KeyEvent;
use crossterm::event::MouseEvent;
pub use goals_pane::{GoalsPane, GoalsPaneState};
pub use interactive::{InteractiveComponent, InteractiveStatefulWidget};
pub use navigation::{
    build_navigation_regions, find_nearest_in_direction, key_to_direction, Direction,
};
pub use selection::{find_region_at, ListSelection, Region, Selection};
pub use theme::Theme;

pub use crate::graph_layout::LayoutSize;

/// Key or mouse event for interactive components.
#[derive(Clone)]
pub enum KeyMouseEvent {
    Key(KeyEvent),
    Mouse(MouseEvent),
}

/// Filter toggles that modes can support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterToggle {
    Instances,
    Inaccessible,
    LetValues,
    ReverseOrder,
}

impl FilterToggle {
    /// Single-character indicator for status bar display.
    pub const fn indicator(self) -> char {
        match self {
            Self::Instances => 'i',
            Self::Inaccessible => 'a',
            Self::LetValues => 'l',
            Self::ReverseOrder => 'r',
        }
    }
}

/// Check if a filter is active in a slice of toggles.
pub fn has_filter(filters: &[FilterToggle], toggle: FilterToggle) -> bool {
    filters.contains(&toggle)
}

/// Create an iterator over hypothesis indices with optional reversal.
pub fn hypothesis_indices(len: usize, filters: &[FilterToggle]) -> Box<dyn Iterator<Item = usize>> {
    let range = 0..len;
    if has_filter(filters, FilterToggle::ReverseOrder) {
        Box::new(range.rev())
    } else {
        Box::new(range)
    }
}
