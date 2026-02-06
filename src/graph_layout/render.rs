use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

use super::{graph::DagGraph, layout::DagLayout, viewport};

pub fn render_edges<N>(
    dag: &DagGraph<N>,
    layout: &DagLayout,
    scroll_x: i32,
    scroll_y: i32,
    top_down: bool,
    viewport: Rect,
    buf: &mut Buffer,
) {
    EdgeRenderer {
        scroll_x,
        scroll_y,
        top_down,
    }
    .render(dag, layout, viewport, buf);
}

pub fn render_scrollbars(
    buf: &mut Buffer,
    area: Rect,
    content_width: i32,
    content_height: i32,
    scroll_x: i32,
    scroll_y: i32,
) {
    if viewport::needs_vertical_scroll(content_height, area) && area.height > 2 {
        let sb_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y,
            1,
            area.height,
        );
        let mut sb_state = ScrollbarState::new(content_height.max(0) as usize)
            .position(scroll_y.max(0) as usize)
            .viewport_content_length(area.height as usize);
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .render(sb_area, buf, &mut sb_state);
    }

    if viewport::needs_horizontal_scroll(content_width, area) && area.width > 2 {
        let sb_area = Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width.saturating_sub(1),
            1,
        );
        let mut sb_state = ScrollbarState::new(content_width.max(0) as usize)
            .position(scroll_x.max(0) as usize)
            .viewport_content_length(area.width as usize);
        Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .begin_symbol(Some("◀"))
            .end_symbol(Some("▶"))
            .render(sb_area, buf, &mut sb_state);
    }
}

struct EdgeRenderer {
    scroll_x: i32,
    scroll_y: i32,
    top_down: bool,
}

impl EdgeRenderer {
    fn render<N>(&self, dag: &DagGraph<N>, layout: &DagLayout, viewport: Rect, buf: &mut Buffer) {
        // Collect all edges with their routing info
        let mut edges_to_draw: Vec<_> = layout
            .nodes
            .iter()
            .flat_map(|pos| {
                let parent_x = pos.x + i32::from(pos.width) / 2;
                let parent_y = if self.top_down {
                    pos.y + i32::from(pos.height)
                } else {
                    pos.y
                };

                dag.edges_from(pos.index).filter_map(move |(child_idx, edge_data)| {
                    let child_pos = layout.find(child_idx)?;
                    let child_x = child_pos.x + i32::from(child_pos.width) / 2;
                    let child_y = if self.top_down {
                        child_pos.y
                    } else {
                        child_pos.y + i32::from(child_pos.height)
                    };
                    Some((parent_x, parent_y, child_x, child_y, edge_data))
                })
            })
            .collect();

        // Sort edges by their routing level and horizontal position
        // Include X midpoint in sort to group only edges that actually overlap horizontally
        edges_to_draw.sort_by_key(|(px, py, cx, cy, _)| {
            let x_mid = (*px + *cx) / 2;
            (*py, *cy, x_mid / 15) // Bucket by X to group only nearby edges
        });

        // Assign routing channel offsets to prevent overlap
        // Only edges at similar horizontal positions get grouped and offset
        let mut prev_routing_key = (i32::MIN, i32::MIN, i32::MIN);
        let mut channel_count = 0i32;

        for (parent_x, parent_y, child_x, child_y, edge_data) in edges_to_draw {
            let x_mid = (parent_x + child_x) / 2;
            let routing_key = (parent_y, child_y, x_mid / 15);
            if routing_key == prev_routing_key {
                channel_count += 1;
            } else {
                channel_count = 0;
                prev_routing_key = routing_key;
            }
            // Alternate X offsets: 0, 1, -1, 2, -2, ...
            let x_offset = if channel_count == 0 {
                0
            } else {
                let half = (channel_count + 1) / 2;
                if channel_count % 2 == 1 { half } else { -half }
            };
            // Y offset: only for overlapping edges (channel_count > 0), shift upward
            let y_offset = -channel_count;

            let style = edge_data.style.unwrap_or_default();
            self.draw_edge_with_channel(
                parent_x,
                parent_y,
                child_x,
                child_y,
                x_offset,
                y_offset,
                style,
                edge_data.label.as_deref(),
                viewport,
                buf,
            );
        }
    }

    fn draw_edge_with_channel(
        &self,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        x_offset: i32,
        y_offset: i32,
        style: Style,
        label: Option<&str>,
        viewport: Rect,
        buf: &mut Buffer,
    ) {
        // Apply X offset (alternating left/right) and Y offset (upward, away from boxes)
        let base_mid_y = i32::midpoint(from_y, to_y);
        let mid_y = base_mid_y + y_offset;

        if from_x == to_x {
            // Straight vertical edge - apply horizontal offset to spread parallel edges
            let offset_x = from_x + x_offset;
            self.draw_vertical_line(offset_x, from_y, to_y, style, '│', viewport, buf);
        } else {
            // Edge needs horizontal routing
            // Apply X offset to spread edges horizontally
            let offset_from_x = from_x + x_offset;
            let offset_to_x = to_x + x_offset;
            let going_down = from_y < to_y;

            // Vertical line from source to routing level
            if going_down {
                self.draw_vertical_line(offset_from_x, from_y, mid_y - 1, style, '│', viewport, buf);
            } else {
                self.draw_vertical_line(offset_from_x, from_y, mid_y + 1, style, '│', viewport, buf);
            }

            // Horizontal line at routing level
            let (start_x, end_x) = if offset_from_x < offset_to_x {
                (offset_from_x + 1, offset_to_x - 1)
            } else {
                (offset_to_x + 1, offset_from_x - 1)
            };
            if start_x <= end_x {
                self.draw_horizontal_line(start_x, end_x, mid_y, style, '─', viewport, buf);
            }

            // Corner characters
            let (from_corner, to_corner) = match (self.top_down, offset_from_x < offset_to_x) {
                (true, true) => ('╰', '╮'),
                (true, false) => ('╯', '╭'),
                (false, true) => ('╭', '╯'),
                (false, false) => ('╮', '╰'),
            };
            self.draw_char(offset_from_x, mid_y, style, from_corner, viewport, buf);
            self.draw_char(offset_to_x, mid_y, style, to_corner, viewport, buf);

            // Vertical line from routing level to target
            if going_down {
                self.draw_vertical_line(offset_to_x, mid_y + 1, to_y, style, '│', viewport, buf);
            } else {
                self.draw_vertical_line(offset_to_x, mid_y - 1, to_y, style, '│', viewport, buf);
            }
        }

        // Draw edge label at the midpoint
        if let Some(label_text) = label {
            let label_len = label_text.chars().count() as i32;
            let (label_x, label_y) = if from_x == to_x {
                (from_x + 1, mid_y)
            } else {
                let center_x = i32::midpoint(from_x, to_x);
                (center_x - label_len / 2, mid_y)
            };
            self.draw_text(label_x, label_y, style, label_text, viewport, buf);
        }
    }

    fn draw_char(&self, vx: i32, vy: i32, style: Style, ch: char, viewport: Rect, buf: &mut Buffer) {
        let screen_x = vx - self.scroll_x + i32::from(viewport.x);
        let screen_y = vy - self.scroll_y + i32::from(viewport.y);

        if screen_x < i32::from(viewport.x)
            || screen_x >= i32::from(viewport.x + viewport.width)
            || screen_y < i32::from(viewport.y)
            || screen_y >= i32::from(viewport.y + viewport.height)
        {
            return;
        }

        if let Some(cell) = buf.cell_mut((screen_x as u16, screen_y as u16)) {
            cell.set_char(ch).set_style(style);
        }
    }

    fn draw_text(&self, vx: i32, vy: i32, style: Style, text: &str, viewport: Rect, buf: &mut Buffer) {
        for (i, ch) in text.chars().enumerate() {
            self.draw_char(vx + i as i32, vy, style, ch, viewport, buf);
        }
    }

    fn draw_vertical_line(
        &self,
        x: i32,
        y1: i32,
        y2: i32,
        style: Style,
        ch: char,
        viewport: Rect,
        buf: &mut Buffer,
    ) {
        let (start, end) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
        for y in start..=end {
            self.draw_char(x, y, style, ch, viewport, buf);
        }
    }

    fn draw_horizontal_line(
        &self,
        x1: i32,
        x2: i32,
        y: i32,
        style: Style,
        ch: char,
        viewport: Rect,
        buf: &mut Buffer,
    ) {
        for x in x1..=x2 {
            self.draw_char(x, y, style, ch, viewport, buf);
        }
    }
}
