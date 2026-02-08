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

/// Hypothesis filter settings.
#[derive(Debug, Clone, Copy, Default)]
pub struct HypothesisFilters {
    pub hide_instances: bool,
    pub hide_inaccessible: bool,
    pub hide_let_values: bool,
    pub reverse_order: bool,
}

/// Filter toggles that modes can support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterToggle {
    Instances,
    Inaccessible,
    LetValues,
    ReverseOrder,
}

/// Create an iterator over hypothesis indices with optional reversal.
pub fn hypothesis_indices(len: usize, reverse: bool) -> Box<dyn Iterator<Item = usize>> {
    let range = 0..len;
    if reverse {
        Box::new(range.rev())
    } else {
        Box::new(range)
    }
}
