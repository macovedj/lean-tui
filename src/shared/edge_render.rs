//! Shared edge rendering for DAG visualizations.

use ratatui::{buffer::Buffer, layout::Rect, style::Style};

use super::{DagLayout, VirtualCanvas};
use crate::protocol::{Dag, DagNode};

/// Context for drawing edges on a virtual canvas.
pub struct EdgeDrawContext<'a> {
    buf: &'a mut Buffer,
    canvas: &'a VirtualCanvas,
    viewport: Rect,
}

impl EdgeDrawContext<'_> {
    fn draw_char(&mut self, vx: i32, vy: i32, style: Style, ch: char) {
        let screen_x = vx - self.canvas.scroll_x + i32::from(self.viewport.x);
        let screen_y = vy - self.canvas.scroll_y + i32::from(self.viewport.y);

        if screen_x < i32::from(self.viewport.x)
            || screen_x >= i32::from(self.viewport.x + self.viewport.width)
            || screen_y < i32::from(self.viewport.y)
            || screen_y >= i32::from(self.viewport.y + self.viewport.height)
        {
            return;
        }

        #[allow(clippy::cast_sign_loss)]
        if let Some(cell) = self.buf.cell_mut((screen_x as u16, screen_y as u16)) {
            cell.set_char(ch).set_style(style);
        }
    }

    fn draw_text(&mut self, vx: i32, vy: i32, style: Style, text: &str) {
        for (i, ch) in text.chars().enumerate() {
            self.draw_char(vx + i as i32, vy, style, ch);
        }
    }

    fn draw_vertical_line(&mut self, x: i32, y1: i32, y2: i32, style: Style, ch: char) {
        let (start, end) = if y1 < y2 { (y1, y2) } else { (y2, y1) };
        for y in start..=end {
            self.draw_char(x, y, style, ch);
        }
    }

    fn draw_horizontal_line(&mut self, x1: i32, x2: i32, y: i32, style: Style, ch: char) {
        for x in x1..=x2 {
            self.draw_char(x, y, style, ch);
        }
    }

    /// Draw an edge with optional label.
    pub fn draw_edge(
        &mut self,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        style: Style,
        top_down: bool,
        label: Option<&str>,
    ) {
        // For same-column edges, draw a simple vertical line
        if from_x == to_x {
            self.draw_vertical_line(from_x, from_y, to_y, style, '│');
        } else {
            // For different columns, draw an elbow with curved corners
            let mid_y = i32::midpoint(from_y, to_y);
            let going_down = from_y < to_y;

            // Vertical from parent to midpoint (excluding mid_y where corner will be)
            if going_down {
                self.draw_vertical_line(from_x, from_y, mid_y - 1, style, '│');
            } else {
                self.draw_vertical_line(from_x, from_y, mid_y + 1, style, '│');
            }

            // Horizontal at midpoint (excluding corner positions)
            let (start_x, end_x) = if from_x < to_x {
                (from_x + 1, to_x - 1)
            } else {
                (to_x + 1, from_x - 1)
            };
            if start_x <= end_x {
                self.draw_horizontal_line(start_x, end_x, mid_y, style, '─');
            }

            // Curved corners - selection depends on both direction and left/right
            let (from_corner, to_corner) = match (top_down, from_x < to_x) {
                (true, true) => ('╰', '╮'),   // top-down, parent left of child
                (true, false) => ('╯', '╭'),  // top-down, parent right of child
                (false, true) => ('╭', '╯'),  // bottom-up, parent left of child
                (false, false) => ('╮', '╰'), // bottom-up, parent right of child
            };
            self.draw_char(from_x, mid_y, style, from_corner);
            self.draw_char(to_x, mid_y, style, to_corner);

            // Vertical from midpoint to child (excluding mid_y where corner is)
            if going_down {
                self.draw_vertical_line(to_x, mid_y + 1, to_y, style, '│');
            } else {
                self.draw_vertical_line(to_x, mid_y - 1, to_y, style, '│');
            }
        }

        // Draw label near the edge
        if let Some(label_text) = label {
            let label_len = label_text.chars().count() as i32;
            let mid_y = i32::midpoint(from_y, to_y);
            let (label_x, label_y) = if from_x != to_x {
                let center_x = (from_x + to_x) / 2;
                (center_x - label_len / 2, mid_y)
            } else {
                (from_x + 1, mid_y)
            };
            self.draw_text(label_x, label_y, style, label_text);
        }
    }
}

/// Edge styling information returned by the style function.
pub struct EdgeStyle {
    pub style: Style,
    pub label: Option<&'static str>,
}

/// Render edges for a DAG with a per-edge style function.
pub fn render_dag_edges_styled<D, F>(
    dag: &D,
    layout: &DagLayout,
    canvas: &VirtualCanvas,
    viewport: Rect,
    buf: &mut Buffer,
    top_down: bool,
    style_fn: F,
) where
    D: Dag,
    F: Fn(&D::Node, usize) -> EdgeStyle,
{
    let mut ctx = EdgeDrawContext {
        buf,
        canvas,
        viewport,
    };

    for pos in &layout.nodes {
        let Some(node) = dag.get_node(pos.node_id) else {
            continue;
        };

        let parent_x = pos.x + i32::from(pos.width) / 2;
        let parent_y = if top_down {
            pos.y + i32::from(pos.height)
        } else {
            pos.y
        };

        for (idx, child_id) in node.children().iter().enumerate() {
            let Some(child_pos) = layout.find_node(*child_id) else {
                continue;
            };

            let child_x = child_pos.x + i32::from(child_pos.width) / 2;
            let child_y = if top_down {
                child_pos.y
            } else {
                child_pos.y + i32::from(child_pos.height)
            };

            let edge_style = style_fn(node, idx);
            ctx.draw_edge(
                parent_x,
                parent_y,
                child_x,
                child_y,
                edge_style.style,
                top_down,
                edge_style.label,
            );
        }
    }
}

/// Render edges for a DAG with uniform styling.
pub fn render_dag_edges<D>(
    dag: &D,
    layout: &DagLayout,
    canvas: &VirtualCanvas,
    viewport: Rect,
    buf: &mut Buffer,
    top_down: bool,
    edge_style: Style,
) where
    D: Dag,
{
    render_dag_edges_styled(dag, layout, canvas, viewport, buf, top_down, |_, _| {
        EdgeStyle {
            style: edge_style,
            label: None,
        }
    });
}
