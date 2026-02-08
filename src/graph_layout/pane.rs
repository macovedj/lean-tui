use petgraph::graph::NodeIndex;
use ratatui::{buffer::Buffer, layout::Rect};

use super::{
    center_on, clip_to_viewport, render_edges, render_scrollbars, scroll_target, DagGraph,
    DagLayout, LayoutConfig,
};
use crate::{protocol::NodeId, selection::Region};

/// Shared viewport state for scrollable graph panes.
#[derive(Default)]
pub struct GraphViewport {
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub manual_scroll: bool,
    prev_current_node: Option<NodeId>,
    pub layout: DagLayout,
    layout_node_count: usize,
    /// Click regions with screen coordinates (for mouse clicks).
    pub click_regions: Vec<Region>,
    /// Navigation regions with virtual coordinates (for keyboard nav).
    pub navigation_regions: Vec<Region>,
}

impl GraphViewport {
    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        if self.prev_current_node != current_node_id {
            self.prev_current_node = current_node_id;
            self.manual_scroll = false;
        }
    }

    pub fn navigation_regions(&self) -> &[Region] {
        &self.navigation_regions
    }

    /// Returns true if layout was recalculated.
    pub fn update_layout_if_needed<F>(&mut self, node_count: usize, compute_layout: F) -> bool
    where
        F: FnOnce() -> DagLayout,
    {
        if self.layout_node_count == node_count {
            false
        } else {
            self.layout = compute_layout();
            self.layout_node_count = node_count;
            true
        }
    }

    pub fn compute_scroll(
        &mut self,
        selected_idx: Option<NodeIndex>,
        current: Option<NodeIndex>,
        root: Option<NodeIndex>,
        viewport: Rect,
    ) -> (i32, i32) {
        let (scroll_x, scroll_y) = if self.manual_scroll {
            (self.scroll_x, self.scroll_y)
        } else {
            scroll_target(&self.layout, selected_idx, current, root).map_or((0, 0), |t| {
                center_on(
                    t,
                    self.layout.content_width,
                    self.layout.content_height,
                    viewport,
                )
            })
        };
        self.scroll_x = scroll_x;
        self.scroll_y = scroll_y;
        (scroll_x, scroll_y)
    }

    pub fn render_edges<N>(
        &self,
        graph: &DagGraph<N>,
        config: LayoutConfig,
        viewport: Rect,
        buf: &mut Buffer,
    ) {
        render_edges(
            graph,
            &self.layout,
            self.scroll_x,
            self.scroll_y,
            config.top_down,
            viewport,
            buf,
        );
    }

    pub fn render_scrollbars(&self, viewport: Rect, buf: &mut Buffer) {
        render_scrollbars(
            buf,
            viewport,
            self.layout.content_width,
            self.layout.content_height,
            self.scroll_x,
            self.scroll_y,
        );
    }

    pub fn clip_node_to_viewport(
        &self,
        x: i32,
        y: i32,
        width: u16,
        height: u16,
        viewport: Rect,
    ) -> Option<Rect> {
        clip_to_viewport(x, y, width, height, self.scroll_x, self.scroll_y, viewport)
    }

    pub fn clear_click_regions(&mut self) {
        self.click_regions.clear();
    }
}
