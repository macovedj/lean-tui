use petgraph::graph::NodeIndex;
use ratatui::{buffer::Buffer, layout::Rect};

use super::{render_edges, render_scrollbars, DagGraph, DagLayout, NodePosition};
use crate::{
    protocol::{NodeId, TimingDisplay},
    selection::{find_region_at, Region, Selection},
};

/// Shared viewport state for scrollable graph panes.
#[derive(Default)]
pub struct GraphViewport {
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub manual_scroll: bool,
    prev_current_node: Option<NodeId>,
    pub layout: DagLayout,
    /// Click regions with screen coordinates (for mouse clicks).
    pub click_regions: Vec<Region>,
    /// Navigation regions with virtual coordinates (for keyboard nav).
    pub navigation_regions: Vec<Region>,
    pub timing_display: TimingDisplay,
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

    /// Recalculates layout and returns true if it changed.
    pub fn update_layout<F>(&mut self, compute_layout: F) -> bool
    where
        F: FnOnce() -> DagLayout,
    {
        let new_layout = compute_layout();
        let changed = self.layout != new_layout;
        self.layout = new_layout;
        changed
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
        top_down: bool,
        viewport: Rect,
        buf: &mut Buffer,
    ) {
        render_edges(
            graph,
            &self.layout,
            self.scroll_x,
            self.scroll_y,
            top_down,
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

    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        find_region_at(&self.click_regions, x, y)
    }
}

fn scroll_target(
    layout: &DagLayout,
    selected: Option<NodeIndex>,
    current: Option<NodeIndex>,
    root: Option<NodeIndex>,
) -> Option<&NodePosition> {
    selected
        .and_then(|idx| layout.find(idx))
        .or_else(|| current.and_then(|idx| layout.find(idx)))
        .or_else(|| root.and_then(|idx| layout.find(idx)))
}

fn center_on(
    target: &NodePosition,
    content_width: i32,
    content_height: i32,
    viewport: Rect,
) -> (i32, i32) {
    let center_x = target.x + i32::from(target.width) / 2;
    let center_y = target.y + i32::from(target.height) / 2;

    let max_x = (content_width - i32::from(viewport.width)).max(0);
    let max_y = (content_height - i32::from(viewport.height)).max(0);

    let scroll_x = (center_x - i32::from(viewport.width) / 2).clamp(0, max_x);
    let scroll_y = (center_y - i32::from(viewport.height) / 2).clamp(0, max_y);

    (scroll_x, scroll_y)
}

fn clip_to_viewport(
    x: i32,
    y: i32,
    width: u16,
    height: u16,
    scroll_x: i32,
    scroll_y: i32,
    viewport: Rect,
) -> Option<Rect> {
    let rel_x = x - scroll_x;
    let rel_y = y - scroll_y;

    let vp_w = i32::from(viewport.width);
    let vp_h = i32::from(viewport.height);
    let w = i32::from(width);
    let h = i32::from(height);

    if rel_x >= vp_w || rel_y >= vp_h || rel_x + w <= 0 || rel_y + h <= 0 {
        return None;
    }

    let (rx, rw) = clip_axis(rel_x, width, viewport.x, viewport.width);
    let (ry, rh) = clip_axis(rel_y, height, viewport.y, viewport.height);

    (rw > 0 && rh > 0).then_some(Rect::new(rx, ry, rw, rh))
}

fn clip_axis(rel_pos: i32, size: u16, origin: u16, area_size: u16) -> (u16, u16) {
    if rel_pos < 0 {
        let clip = (-rel_pos) as u16;
        (origin, size.saturating_sub(clip).min(area_size))
    } else {
        let pos = rel_pos as u16;
        (origin + pos, size.min(area_size.saturating_sub(pos)))
    }
}
