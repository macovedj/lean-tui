//! Effect graph pane with edge rendering.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

use super::{
    state_node::{EffectStateNode, EffectStateNodeState},
    types::EffectFlowEdgeExt,
};
use crate::{
    protocol::{GenericDag, NodeId},
    shared::{
        build_navigation_regions, calculate_dag_layout, ClickRegion, DagLayout, LayoutConfig,
        NavigationRegion, NodePosition, Selection, VirtualCanvas,
    },
};

struct DrawContext<'a> {
    buf: &'a mut Buffer,
    canvas: &'a VirtualCanvas,
    viewport: Rect,
}

impl DrawContext<'_> {
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
        let cell = self.buf.cell_mut((screen_x as u16, screen_y as u16));
        if let Some(cell) = cell {
            cell.set_char(ch).set_style(style);
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

    fn draw_edge(
        &mut self,
        from_x: i32,
        from_y: i32,
        to_x: i32,
        to_y: i32,
        style: Style,
        top_down: bool,
    ) {
        let mid_y = i32::midpoint(from_y, to_y);

        self.draw_vertical_line(from_x, from_y, mid_y, style, '│');

        if from_x != to_x {
            let (start_x, end_x) = if from_x < to_x {
                (from_x, to_x)
            } else {
                (to_x, from_x)
            };
            self.draw_horizontal_line(start_x, end_x, mid_y, style, '─');

            let left_corner = if from_x < to_x { '└' } else { '┘' };
            let right_corner = if from_x < to_x { '┐' } else { '┌' };
            self.draw_char(from_x, mid_y, style, left_corner);
            self.draw_char(to_x, mid_y, style, right_corner);
        }

        self.draw_vertical_line(to_x, mid_y, to_y, style, '│');

        let arrow = if top_down { '▼' } else { '▲' };
        let arrow_y = if top_down { to_y - 1 } else { to_y + 1 };
        self.draw_char(to_x, arrow_y, style, arrow);
    }
}

/// State for the effect graph pane.
#[derive(Default)]
pub struct GraphPaneState {
    scroll_x: i32,
    scroll_y: i32,
    manual_scroll: bool,
    prev_current_node: Option<NodeId>,
    layout: DagLayout,
    pub stretch_children: bool,
    pub click_regions: Vec<ClickRegion>,
    pub navigation_regions: Vec<NavigationRegion>,
}

impl GraphPaneState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        let point = Rect::new(x, y, 1, 1);
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(point))
            .map(|r| r.selection)
    }

    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        if self.prev_current_node != current_node_id {
            self.prev_current_node = current_node_id;
            self.manual_scroll = false;
        }
    }

    pub fn navigation_regions(&self) -> &[NavigationRegion] {
        &self.navigation_regions
    }

    pub fn toggle_stretch(&mut self) {
        self.stretch_children = !self.stretch_children;
    }

    pub const fn stretch_enabled(&self) -> bool {
        self.stretch_children
    }
}

/// Render the main effect graph with nodes and edges.
pub fn render_graph_pane(
    dag: &GenericDag,
    config: LayoutConfig,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut GraphPaneState,
) {
    state.click_regions.clear();
    state.navigation_regions.clear();

    if dag.is_empty() || dag.root_node_id.is_none() {
        Paragraph::new("No effect flow steps")
            .style(Style::new().fg(Color::DarkGray))
            .render(area, buf);
        return;
    }

    let layout_config = LayoutConfig {
        top_down: config.top_down,
        stretch_children: state.stretch_children,
    };
    state.layout = calculate_dag_layout(dag, layout_config);
    if state.layout.nodes.is_empty() {
        return;
    }

    state.navigation_regions = build_navigation_regions(dag, &state.layout);

    let canvas = create_canvas(state, dag, selection, area);
    state.scroll_x = canvas.scroll_x;
    state.scroll_y = canvas.scroll_y;

    // First pass: render edges between nodes
    render_edges(dag, &state.layout, &canvas, area, buf, config.top_down);

    // Second pass: render nodes on top of edges
    let click_regions: Vec<ClickRegion> = state
        .layout
        .nodes
        .iter()
        .filter_map(|pos| {
            let render_area = canvas.clip_to_viewport(pos.x, pos.y, pos.width, pos.height, area)?;
            let node = dag.get(pos.node_id)?;
            (render_area.width >= 3 && render_area.height >= 3).then_some((render_area, node, pos))
        })
        .flat_map(|(render_area, node, pos)| {
            let mut node_state = EffectStateNodeState::default();
            EffectStateNode::new(
                node,
                dag.is_current(pos.node_id),
                selection,
                config.top_down,
            )
            .render(render_area, buf, &mut node_state);
            node_state.click_regions
        })
        .collect();

    state.click_regions = click_regions;

    render_scrollbars(buf, area, &canvas);
}

fn render_edges(
    dag: &GenericDag,
    layout: &DagLayout,
    canvas: &VirtualCanvas,
    viewport: Rect,
    buf: &mut Buffer,
    top_down: bool,
) {
    let mut ctx = DrawContext {
        buf,
        canvas,
        viewport,
    };

    for pos in &layout.nodes {
        let Some(node) = dag.get(pos.node_id) else {
            continue;
        };

        let parent_x = pos.x + i32::from(pos.width) / 2;
        let parent_y = if top_down {
            pos.y + i32::from(pos.height)
        } else {
            pos.y
        };

        for edge in &node.edges {
            let Some(child_pos) = layout.find_node(edge.target) else {
                continue;
            };

            let child_x = child_pos.x + i32::from(child_pos.width) / 2;
            let child_y = if top_down {
                child_pos.y
            } else {
                child_pos.y + i32::from(child_pos.height)
            };

            let edge_kind = edge.edge_kind();
            let edge_style = Style::new().fg(edge_kind.color());

            ctx.draw_edge(parent_x, parent_y, child_x, child_y, edge_style, top_down);
        }
    }
}

fn create_canvas(
    state: &GraphPaneState,
    dag: &GenericDag,
    selection: Option<Selection>,
    area: Rect,
) -> VirtualCanvas {
    let canvas = VirtualCanvas::new(state.layout.content_width, state.layout.content_height);

    if state.manual_scroll {
        canvas.with_scroll(state.scroll_x, state.scroll_y, area)
    } else {
        find_scroll_target(&state.layout, dag, selection).map_or(canvas, |t| {
            canvas.centered_on(t.x, t.y, t.width, t.height, area)
        })
    }
}

fn find_scroll_target<'a>(
    layout: &'a DagLayout,
    dag: &GenericDag,
    selection: Option<Selection>,
) -> Option<&'a NodePosition> {
    let selected_node_id = selection.and_then(|sel| match sel {
        Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. } => Some(node_id),
        _ => None,
    });

    selected_node_id
        .and_then(|id| layout.find_node(id))
        .or_else(|| dag.current_node_id.and_then(|id| layout.find_node(id)))
        .or_else(|| dag.root_node_id.and_then(|id| layout.find_node(id)))
}

fn render_scrollbars(buf: &mut Buffer, area: Rect, canvas: &VirtualCanvas) {
    if canvas.needs_vertical_scroll(area) && area.height > 2 {
        let sb_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y,
            1,
            area.height,
        );
        let mut sb_state = ScrollbarState::new(canvas.content_height.max(0) as usize)
            .position(canvas.scroll_y.max(0) as usize)
            .viewport_content_length(area.height as usize);
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .render(sb_area, buf, &mut sb_state);
    }

    if canvas.needs_horizontal_scroll(area) && area.width > 2 {
        let sb_area = Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width.saturating_sub(1),
            1,
        );
        let mut sb_state = ScrollbarState::new(canvas.content_width.max(0) as usize)
            .position(canvas.scroll_x.max(0) as usize)
            .viewport_content_length(area.width as usize);
        Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .begin_symbol(Some("◀"))
            .end_symbol(Some("▶"))
            .render(sb_area, buf, &mut sb_state);
    }
}
