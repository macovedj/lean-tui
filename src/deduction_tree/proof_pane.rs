use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use super::{
    state_node::{StateNode, StateNodeState},
    types::CompleteProofDag,
};
use crate::{
    deduction_tree::TacticProofState,
    graph_layout::{calculate_layout, GraphViewport},
    navigation::build_navigation_regions,
    selection::Selection,
};

/// Proof pane widget.
pub struct ProofPane<'a> {
    dag: &'a CompleteProofDag,
    top_down: bool,
    selection: Option<Selection>,
    current_state: &'a TacticProofState,
}

impl<'a> ProofPane<'a> {
    pub const fn new(
        dag: &'a CompleteProofDag,
        top_down: bool,
        selection: Option<Selection>,
        current_state: &'a TacticProofState,
    ) -> Self {
        Self {
            dag,
            top_down,
            selection,
            current_state,
        }
    }
}

impl StatefulWidget for ProofPane<'_> {
    type State = GraphViewport;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.clear_click_regions();

        if self.dag.is_empty() || self.dag.root_node_id.is_none() {
            Paragraph::new("No proof steps")
                .style(Style::new().fg(Color::DarkGray))
                .render(area, buf);
            return;
        }

        let graph = self.dag.to_graph();

        let layout_changed = state.update_layout_if_needed(self.dag.len(), || {
            let layout_graph = self.dag.to_layout_graph();
            calculate_layout(&layout_graph, self.top_down)
        });

        if layout_changed {
            state.navigation_regions = build_navigation_regions(self.dag, &state.layout);
        }

        if state.layout.nodes.is_empty() {
            return;
        }

        let selected_idx = self.selection.and_then(|sel| match sel {
            Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. } => {
                graph.index_of(node_id)
            }
            _ => None,
        });

        state.compute_scroll(selected_idx, graph.current, graph.root, area);
        state.render_edges(&graph, self.top_down, area, buf);

        for pos in &state.layout.nodes {
            let Some(render_area) =
                state.clip_node_to_viewport(pos.x, pos.y, pos.width, pos.height, area)
            else {
                continue;
            };
            let Some(node) = graph.get(pos.index) else {
                continue;
            };
            if render_area.width < 3 || render_area.height < 3 {
                continue;
            }

            let is_current = graph.is_current(pos.index);
            let override_state = if is_current && !self.current_state.goals.is_empty() {
                Some(self.current_state)
            } else {
                None
            };

            let node_widget = StateNode::new(node, is_current, self.selection, override_state);
            let mut node_state = StateNodeState::default();
            node_widget.render(render_area, buf, &mut node_state);
            state.click_regions.extend(node_state.click_regions);
        }

        state.render_scrollbars(area, buf);
    }
}
