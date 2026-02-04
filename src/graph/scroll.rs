use petgraph::graph::NodeIndex;
use ratatui::layout::Rect;

use super::layout::{DagLayout, NodePosition};

pub fn scroll_target(
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

pub fn center_on(
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

pub fn clip_to_viewport(
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

pub fn needs_horizontal_scroll(content_width: i32, viewport: Rect) -> bool {
    content_width > i32::from(viewport.width)
}

pub fn needs_vertical_scroll(content_height: i32, viewport: Rect) -> bool {
    content_height > i32::from(viewport.height)
}
