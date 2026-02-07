use std::collections::HashMap;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

use super::{graph::DagGraph, layout::DagLayout, viewport};

struct HorizontalSegment {
    mid_y: i32,
    x_left: i32,
    x_right: i32,
}

struct EdgeRoute<'a> {
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    segment: HorizontalSegment,
    style: Style,
    label: Option<&'a str>,
}

fn compute_y_offsets(routes: &[EdgeRoute<'_>]) -> Vec<i32> {
    let mut by_y: HashMap<i32, Vec<(usize, i32, i32)>> = HashMap::new();
    for (idx, route) in routes.iter().enumerate() {
        by_y.entry(route.segment.mid_y).or_default().push((
            idx,
            route.segment.x_left,
            route.segment.x_right,
        ));
    }

    let mut offsets = vec![0i32; routes.len()];

    for segments in by_y.values_mut() {
        segments.sort_by_key(|(_, x_left, _)| *x_left);

        // channels[c] = rightmost x_end currently in channel c
        let mut channels: Vec<i32> = Vec::new();

        for &(idx, x_left, x_right) in segments.iter() {
            let channel = channels
                .iter()
                .position(|&end| end < x_left)
                .unwrap_or_else(|| {
                    channels.push(i32::MIN);
                    channels.len() - 1
                });

            channels[channel] = x_right;
            offsets[idx] = -(channel as i32);
        }
    }

    offsets
}

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

fn parent_y(pos: &super::NodePosition, top_down: bool) -> i32 {
    if top_down {
        pos.y + i32::from(pos.height)
    } else {
        pos.y
    }
}

fn child_y(pos: &super::NodePosition, top_down: bool) -> i32 {
    if top_down {
        pos.y
    } else {
        pos.y + i32::from(pos.height)
    }
}

fn ordered_x_pair(a: i32, b: i32) -> (i32, i32) {
    if a <= b {
        (a, b)
    } else {
        (b, a)
    }
}

struct EdgeRenderer {
    scroll_x: i32,
    scroll_y: i32,
    top_down: bool,
}

impl EdgeRenderer {
    fn render<N>(&self, dag: &DagGraph<N>, layout: &DagLayout, viewport: Rect, buf: &mut Buffer) {
        let routes: Vec<EdgeRoute<'_>> = layout
            .nodes
            .iter()
            .flat_map(|pos| self.routes_from_node(pos, dag, layout))
            .collect();

        let y_offsets = compute_y_offsets(&routes);

        for (route, &y_offset) in routes.iter().zip(y_offsets.iter()) {
            self.draw_edge(route, y_offset, viewport, buf);
        }
    }

    fn routes_from_node<'a, N>(
        &self,
        pos: &super::NodePosition,
        dag: &'a DagGraph<N>,
        layout: &'a DagLayout,
    ) -> impl Iterator<Item = EdgeRoute<'a>> + 'a {
        let parent_x = pos.x + i32::from(pos.width) / 2;
        let parent_y_val = parent_y(pos, self.top_down);
        let top_down = self.top_down;

        dag.edges_from(pos.index)
            .filter_map(move |(child_idx, edge_data)| {
                let child_pos = layout.find(child_idx)?;
                let child_x = child_pos.x + i32::from(child_pos.width) / 2;
                let child_y_val = child_y(child_pos, top_down);
                let mid_y = i32::midpoint(parent_y_val, child_y_val);
                let (x_left, x_right) = ordered_x_pair(parent_x, child_x);

                Some(EdgeRoute {
                    from_x: parent_x,
                    from_y: parent_y_val,
                    to_x: child_x,
                    to_y: child_y_val,
                    segment: HorizontalSegment {
                        mid_y,
                        x_left,
                        x_right,
                    },
                    style: edge_data.style.unwrap_or_default(),
                    label: edge_data.label.as_deref(),
                })
            })
    }

    fn draw_edge(&self, route: &EdgeRoute<'_>, y_offset: i32, viewport: Rect, buf: &mut Buffer) {
        let EdgeRoute {
            from_x,
            from_y,
            to_x,
            to_y,
            style,
            label,
            ..
        } = *route;
        let mid_y = route.segment.mid_y + y_offset;

        if from_x == to_x {
            self.draw_vertical_line(from_x, from_y, to_y, style, '│', viewport, buf);
        } else {
            let going_down = from_y < to_y;

            if going_down {
                self.draw_vertical_line(from_x, from_y, mid_y - 1, style, '│', viewport, buf);
            } else {
                self.draw_vertical_line(from_x, from_y, mid_y + 1, style, '│', viewport, buf);
            }

            let (start_x, end_x) = if from_x < to_x {
                (from_x + 1, to_x - 1)
            } else {
                (to_x + 1, from_x - 1)
            };
            if start_x <= end_x {
                self.draw_horizontal_line(start_x, end_x, mid_y, style, '─', viewport, buf);
            }

            let (from_corner, to_corner) = match (self.top_down, from_x < to_x) {
                (true, true) => ('╰', '╮'),
                (true, false) => ('╯', '╭'),
                (false, true) => ('╭', '╯'),
                (false, false) => ('╮', '╰'),
            };
            self.draw_char(from_x, mid_y, style, from_corner, viewport, buf);
            self.draw_char(to_x, mid_y, style, to_corner, viewport, buf);

            if going_down {
                self.draw_vertical_line(to_x, mid_y + 1, to_y, style, '│', viewport, buf);
            } else {
                self.draw_vertical_line(to_x, mid_y - 1, to_y, style, '│', viewport, buf);
            }
        }

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

    fn draw_char(
        &self,
        vx: i32,
        vy: i32,
        style: Style,
        ch: char,
        viewport: Rect,
        buf: &mut Buffer,
    ) {
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

    fn draw_text(
        &self,
        vx: i32,
        vy: i32,
        style: Style,
        text: &str,
        viewport: Rect,
        buf: &mut Buffer,
    ) {
        for (i, ch) in text.chars().enumerate() {
            self.draw_char(vx + i as i32, vy, style, ch, viewport, buf);
        }
    }

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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
