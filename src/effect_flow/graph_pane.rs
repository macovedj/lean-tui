//! Effect graph pane with edge rendering.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};
use tracing::debug;

use super::{
    state_node::{EffectStateNode, EffectStateNodeState},
    types::EffectFlowDag,
};
use crate::{
    protocol::NodeId,
    shared::{
        build_navigation_regions, calculate_dag_layout, render_dag_edges_styled, ClickRegion,
        DagLayout, EdgeStyle, LayoutConfig, NavigationRegion, NodePosition, Selection,
        VirtualCanvas,
    },
};

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
}

/// Render the main effect graph with nodes and edges.
pub fn render_graph_pane(
    dag: &EffectFlowDag,
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
        h_gap: config.h_gap,
        v_gap: config.v_gap,
    };

    // Log DAG structure for debugging
    debug!("=== DAG structure for graph pane ===");
    debug!("root_node_id: {:?}", dag.root_node_id);
    for node in &dag.nodes {
        debug!(
            "  node {}: {} children: {:?}, edges: {:?}",
            node.id,
            node.edges.len(),
            node.edges.iter().map(|e| e.target).collect::<Vec<_>>(),
            node.edges
                .iter()
                .map(|e| (e.target, e.kind.as_deref()))
                .collect::<Vec<_>>()
        );
    }
    debug!("=== end DAG structure ===");

    state.layout = calculate_dag_layout(dag, layout_config);
    if state.layout.nodes.is_empty() {
        return;
    }

    state.navigation_regions = build_navigation_regions(dag, &state.layout);

    let canvas = create_canvas(state, dag, selection, area);
    state.scroll_x = canvas.scroll_x;
    state.scroll_y = canvas.scroll_y;

    // First pass: render edges between nodes
    render_dag_edges_styled(
        dag,
        &state.layout,
        &canvas,
        area,
        buf,
        config.top_down,
        |node, idx| {
            let edge = &node.edges[idx];
            let edge_kind = edge.effect_edge_kind();
            EdgeStyle {
                style: Style::new().fg(edge_kind.color()),
                label: edge_kind.label(),
            }
        },
    );

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
            let is_current = dag.is_current(pos.node_id);
            let is_selected = matches!(selection, Some(Selection::Goal { node_id, .. }) if node_id == pos.node_id);
            let mut node_state = EffectStateNodeState::default();
            EffectStateNode::new(node, is_current, is_selected)
                .render(render_area, buf, &mut node_state);
            node_state.click_regions
        })
        .collect();

    state.click_regions = click_regions;

    render_scrollbars(buf, area, &canvas);
}

fn create_canvas(
    state: &GraphPaneState,
    dag: &EffectFlowDag,
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
    dag: &EffectFlowDag,
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
