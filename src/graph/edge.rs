use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use super::{layout::DagLayout, types::DagGraph};

pub struct EdgeRenderer {
    pub scroll_x: i32,
    pub scroll_y: i32,
    pub top_down: bool,
}

impl EdgeRenderer {
    pub fn render<N>(
        &self,
        dag: &DagGraph<N>,
        layout: &DagLayout,
        viewport: Rect,
        buf: &mut Buffer,
    ) {
        for pos in &layout.nodes {
            let parent_x = pos.x + i32::from(pos.width) / 2;
            let parent_y = if self.top_down {
                pos.y + i32::from(pos.height)
            } else {
                pos.y
            };

            for (child_idx, edge_data) in dag.edges_from(pos.index) {
                let Some(child_pos) = layout.find(child_idx) else {
                    continue;
                };

                let child_x = child_pos.x + i32::from(child_pos.width) / 2;
                let child_y = if self.top_down {
                    child_pos.y
                } else {
                    child_pos.y + i32::from(child_pos.height)
                };

                let style = edge_data.style.unwrap_or_default();
                self.draw_edge(
                    parent_x,
                    parent_y,
                    child_x,
                    child_y,
                    style,
                    edge_data.label,
                    viewport,
                    buf,
                );
            }
        }
    }

    fn draw_edge(
        &self,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        style: Style,
        label: Option<&str>,
        viewport: Rect,
        buf: &mut Buffer,
    ) {
        if from_x == to_x {
            self.draw_vertical_line(from_x, from_y, to_y, style, '│', viewport, buf);
        } else {
            let mid_y = i32::midpoint(from_y, to_y);
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
            let mid_y = i32::midpoint(from_y, to_y);
            let (label_x, label_y) = if from_x != to_x {
                let center_x = (from_x + to_x) / 2;
                (center_x - label_len / 2, mid_y)
            } else {
                (from_x + 1, mid_y)
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
