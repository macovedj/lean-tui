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
