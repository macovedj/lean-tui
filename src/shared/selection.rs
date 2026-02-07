//! Reusable selection state and click region handling.

use ratatui::layout::Rect;

use crate::protocol::NodeId;

/// Unified selection type for all display modes.
/// All selections reference data in `ProofDag`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// Initial hypothesis from theorem statement.
    InitialHyp { hyp_idx: usize },
    /// Hypothesis at a proof step (`node_id` indexes into `ProofDag`).
    Hyp { node_id: NodeId, hyp_idx: usize },
    /// Goal at a proof step.
    Goal { node_id: NodeId, goal_idx: usize },
    /// The theorem conclusion.
    Theorem,
    /// Error diagnostic at a proof step.
    Error { node_id: NodeId, error_idx: usize },
}

/// A region mapped to a selection. Uses i32 coordinates to support
/// both screen coordinates and virtual (scrollable) coordinates.
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
    pub selection: Selection,
}

impl Region {
    /// Create a region from screen coordinates.
    pub fn from_rect(area: Rect, selection: Selection) -> Self {
        Self {
            x: i32::from(area.x),
            y: i32::from(area.y),
            width: area.width,
            height: area.height,
            selection,
        }
    }

    /// Check if screen coordinates (x, y) fall within this region.
    pub fn contains(&self, x: u16, y: u16) -> bool {
        let px = i32::from(x);
        let py = i32::from(y);
        px >= self.x
            && px < self.x + i32::from(self.width)
            && py >= self.y
            && py < self.y + i32::from(self.height)
    }
}

/// Find a selection at screen coordinates (x, y) in a slice of regions.
pub fn find_region_at(regions: &[Region], x: u16, y: u16) -> Option<Selection> {
    regions
        .iter()
        .find(|r| r.contains(x, y))
        .map(|r| r.selection)
}

/// Manages selection state for linear (list-based) navigation.
#[derive(Debug, Default)]
pub struct ListSelection {
    selected_index: Option<usize>,
    regions: Vec<Region>,
}

impl ListSelection {
    pub fn clear_regions(&mut self) {
        self.regions.clear();
    }

    pub fn add_region(&mut self, area: Rect, selection: Selection) {
        self.regions.push(Region::from_rect(area, selection));
    }

    pub fn reset(&mut self, item_count: usize) {
        self.selected_index = (item_count > 0).then_some(0);
    }

    pub fn select_previous(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.selected_index = Some(self.selected_index.map_or(0, |i| i.saturating_sub(1)));
    }

    pub const fn select_next(&mut self, item_count: usize) {
        if item_count == 0 {
            return;
        }
        self.selected_index = Some(match self.selected_index {
            Some(i) if i < item_count - 1 => i + 1,
            Some(i) => i,
            None => 0,
        });
    }

    pub fn current_selection<'a>(&self, items: &'a [Selection]) -> Option<&'a Selection> {
        self.selected_index.and_then(|i| items.get(i))
    }

    /// Handle a click at (x, y). Returns true if selection changed.
    pub fn handle_click(&mut self, x: u16, y: u16, items: &[Selection]) -> bool {
        let Some(sel) = find_region_at(&self.regions, x, y) else {
            return false;
        };

        if let Some(idx) = items.iter().position(|i| *i == sel) {
            self.selected_index = Some(idx);
            return true;
        }
        false
    }
}
