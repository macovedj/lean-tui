use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use super::{
    state_node::{FunctionalStateNode, FunctionalStateNodeState},
    types::DataFlowDag,
};
use crate::{
    graph_layout::{calculate_layout, GraphViewport},
    navigation::build_navigation_regions,
    protocol::NodeId,
    selection::{find_region_at, Region, Selection},
};

/// State for the functional tableau layout.
#[derive(Default)]
pub struct FunctionalTableauState {
    pub proof: GraphViewport,
}

impl FunctionalTableauState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        find_region_at(self.proof.click_regions.as_slice(), x, y)
    }

    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        self.proof.update_current_node(current_node_id);
    }

    /// Get all navigation regions.
    pub fn navigation_regions(&self) -> Vec<Region> {
        self.proof.navigation_regions().to_vec()
    }
}

/// Functional tableau layout widget.
pub struct FunctionalTableauLayout<'a> {
    dag: &'a DataFlowDag,
    top_down: bool,
    selection: Option<Selection>,
}

impl<'a> FunctionalTableauLayout<'a> {
    pub fn new(dag: &'a DataFlowDag, top_down: bool, selection: Option<Selection>) -> Self {
        Self {
            dag,
            top_down,
            selection,
        }
    }
}

impl StatefulWidget for FunctionalTableauLayout<'_> {
    type State = FunctionalTableauState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_proof_pane(
            self.dag,
            self.top_down,
            self.selection,
            area,
            buf,
            &mut state.proof,
        );
    }
}

fn render_proof_pane(
    dag: &DataFlowDag,
    top_down: bool,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut GraphViewport,
) {
    state.clear_click_regions();

    if dag.is_empty() || dag.root_node_id.is_none() {
        Paragraph::new("No expression steps")
            .style(Style::new().fg(Color::DarkGray))
            .render(area, buf);
        return;
    }

    let graph = dag.to_graph();

    let layout_changed = state.update_layout(|| {
        let layout_graph = dag.to_layout_graph();
        calculate_layout(&layout_graph, top_down)
    });

    if layout_changed {
        state.navigation_regions = build_navigation_regions(dag, &state.layout);
    }

    if state.layout.nodes.is_empty() {
        return;
    }

    let selected_idx = selection.and_then(|sel| match sel {
        Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. } => graph.index_of(node_id),
        _ => None,
    });

    state.compute_scroll(selected_idx, graph.current, graph.root, area);
    state.render_edges(&graph, top_down, area, buf);

    let click_regions: Vec<_> = state
        .layout
        .nodes
        .iter()
        .filter_map(|pos| {
            let render_area =
                state.clip_node_to_viewport(pos.x, pos.y, pos.width, pos.height, area)?;
            let node = graph.get(pos.index)?;
            (render_area.width >= 3 && render_area.height >= 3).then_some((render_area, node, pos))
        })
        .flat_map(|(render_area, node, pos)| {
            let is_current = graph.is_current(pos.index);
            let mut node_state = FunctionalStateNodeState::default();
            FunctionalStateNode::new(node, is_current, selection, top_down).render(
                render_area,
                buf,
                &mut node_state,
            );
            node_state.click_regions
        })
        .collect();
    state.click_regions.extend(click_regions);
    state.render_scrollbars(area, buf);
}
