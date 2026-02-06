use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use super::{
    state_node::{EffectStateNode, EffectStateNodeState},
    types::EffectFlowDag,
};
use crate::{
    graph::{
        calculate_layout, center_on, clip_to_viewport, render_edges, render_scrollbars,
        scroll_target, DagLayout, LayoutConfig,
    },
    protocol::NodeId,
    shared::{ClickRegion, NavigationRegion, Selection},
};

#[derive(Default)]
pub struct GraphPaneState {
    scroll_x: i32,
    scroll_y: i32,
    manual_scroll: bool,
    prev_current_node: Option<NodeId>,
    layout: DagLayout,
    layout_node_count: usize,
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

    let graph = dag.to_graph();

    // Only recalculate layout when node count changes
    if state.layout_node_count != dag.len() {
        let layout_graph = dag.to_layout_graph();
        let layout_config = LayoutConfig {
            top_down: config.top_down,
            ..Default::default()
        };
        state.layout = calculate_layout(&layout_graph, layout_config);
        state.layout_node_count = dag.len();
    }

    if state.layout.nodes.is_empty() {
        return;
    }

    let selected_idx = selection.and_then(|sel| match sel {
        Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. } => graph.index_of(node_id),
        _ => None,
    });

    let (scroll_x, scroll_y) = if state.manual_scroll {
        (state.scroll_x, state.scroll_y)
    } else {
        scroll_target(&state.layout, selected_idx, graph.current, graph.root)
            .map(|t| {
                center_on(
                    t,
                    state.layout.content_width,
                    state.layout.content_height,
                    area,
                )
            })
            .unwrap_or((0, 0))
    };
    state.scroll_x = scroll_x;
    state.scroll_y = scroll_y;

    render_edges(
        &graph,
        &state.layout,
        scroll_x,
        scroll_y,
        config.top_down,
        area,
        buf,
    );

    let click_regions: Vec<ClickRegion> = state
        .layout
        .nodes
        .iter()
        .filter_map(|pos| {
            let render_area = clip_to_viewport(
                pos.x, pos.y, pos.width, pos.height, scroll_x, scroll_y, area,
            )?;
            let node = graph.get(pos.index)?;
            (render_area.width >= 3 && render_area.height >= 3).then_some((render_area, node, pos))
        })
        .flat_map(|(render_area, node, pos)| {
            let is_current = graph.is_current(pos.index);
            let is_selected =
                matches!(selection, Some(Selection::Goal { node_id, .. }) if node_id == node.id);
            let mut node_state = EffectStateNodeState::default();
            EffectStateNode::new(node, is_current, is_selected).render(
                render_area,
                buf,
                &mut node_state,
            );
            node_state.click_regions
        })
        .collect();

    state.click_regions = click_regions;

    render_scrollbars(
        buf,
        area,
        state.layout.content_width,
        state.layout.content_height,
        scroll_x,
        scroll_y,
    );
}
