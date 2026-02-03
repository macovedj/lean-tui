pub mod canvas;
pub mod dag_layout;
pub mod diff_text;
pub mod error_banner;
pub mod goal_section;
pub mod help_menu;
pub mod hyp_layer;
pub mod hyp_section;
pub mod interactive;
pub mod legend;
pub mod navigation;
pub mod render_helpers;
pub mod selection;
pub mod status_bar;
pub mod tactic_row;
pub mod theme;
pub mod welcome;
pub mod worker_selector;

pub use canvas::VirtualCanvas;
pub use crossterm::event::KeyEvent;
use crossterm::event::MouseEvent;
pub use dag_layout::{calculate_dag_layout, DagLayout, LayoutConfig, LayoutSize, NodePosition};
pub use interactive::{InteractiveComponent, InteractiveStatefulWidget};
pub use navigation::{
    build_navigation_regions, find_nearest_in_direction, Direction, NavigationRegion,
};
pub use selection::{ClickRegion, Selection};
pub use theme::Theme;

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

impl HypothesisFilters {
    /// Toggle a filter setting.
    pub const fn toggle(&mut self, filter: FilterToggle) {
        match filter {
            FilterToggle::Instances => self.hide_instances = !self.hide_instances,
            FilterToggle::Inaccessible => self.hide_inaccessible = !self.hide_inaccessible,
            FilterToggle::LetValues => self.hide_let_values = !self.hide_let_values,
            FilterToggle::ReverseOrder => self.reverse_order = !self.reverse_order,
        }
    }
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

/// Truncate a string to max length with ellipsis (character-safe).
pub fn truncate_str(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        let truncated: String = s.chars().take(max_len - 3).collect();
        format!("{truncated}...")
    }
}
