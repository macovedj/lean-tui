use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use super::graph::EffectFlowDag;
use crate::{
    graph_layout::{calculate_layout, GraphViewport},
    navigation::build_navigation_regions,
    selection::Selection,
    widgets::node_content::{NodeContentWidget, NodeWidgetState},
};

pub struct EffectFlowLayout<'a> {
    dag: &'a EffectFlowDag,
    selection: Option<Selection>,
    expanded: bool,
}

impl<'a> EffectFlowLayout<'a> {
    pub fn new(dag: &'a EffectFlowDag, selection: Option<Selection>, expanded: bool) -> Self {
        Self {
            dag,
            selection,
            expanded,
        }
    }
}

impl StatefulWidget for EffectFlowLayout<'_> {
    type State = GraphViewport;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.clear_click_regions();

        if self.dag.is_empty() || self.dag.root_node_id.is_none() {
            Paragraph::new("No effect flow steps")
                .style(Style::new().fg(Color::DarkGray))
                .render(area, buf);
            return;
        }

        let top_down = true;
        let graph = self.dag.to_graph_filtered(self.expanded);

        let layout_changed = state.update_layout(|| {
            let layout_graph = self.dag.to_layout_graph_filtered(self.expanded);
            calculate_layout(&layout_graph, top_down)
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
        state.render_edges(&graph, top_down, area, buf);

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
            let content = node.build_content(self.selection, is_current);
            let style = node.build_node_style(is_current);
            let mut node_state = NodeWidgetState::default();
            NodeContentWidget { content, style }.render(render_area, buf, &mut node_state);
            state.click_regions.extend(node_state.click_regions);
        }

        state.render_scrollbars(area, buf);
    }
}
