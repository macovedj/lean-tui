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
    graph::{
        calculate_layout, center_on, clip_to_viewport, render_edges, render_scrollbars,
        scroll_target, DagLayout, LayoutConfig,
    },
    protocol::NodeId,
    shared::{build_navigation_regions, ClickRegion, LayoutSize, NavigationRegion, Selection},
};

/// State for the proof pane widget.
#[derive(Default)]
pub struct ProofPaneState {
    scroll_x: i32,
    scroll_y: i32,
    manual_scroll: bool,
    prev_current_node: Option<NodeId>,
    layout: DagLayout,
    pub click_regions: Vec<ClickRegion>,
    pub navigation_regions: Vec<NavigationRegion>,
}

impl ProofPaneState {
    pub fn update_current_node(&mut self, current_node: Option<NodeId>) {
        if current_node != self.prev_current_node {
            self.manual_scroll = false;
            self.prev_current_node = current_node;
        }
    }

    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(Rect::new(x, y, 1, 1)))
            .map(|r| r.selection)
    }

    pub fn navigation_regions(&self) -> &[NavigationRegion] {
        &self.navigation_regions
    }
}

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
    type State = ProofPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();
        state.navigation_regions.clear();

        if self.dag.is_empty() || self.dag.root_node_id.is_none() {
            Paragraph::new("No proof steps")
                .style(Style::new().fg(Color::DarkGray))
                .render(area, buf);
            return;
        }

        let graph = self.dag.to_graph();

        let config = LayoutConfig {
            top_down: self.top_down,
            stretch_children: false,
            h_gap: 1,
            v_gap: 1,
        };

        state.layout = calculate_layout(&graph, config, |node| node.layout_size());
        if state.layout.nodes.is_empty() {
            return;
        }

        state.navigation_regions = build_navigation_regions(self.dag, &state.layout);

        let selected_idx = self.selection.and_then(|sel| match sel {
            Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. } => {
                graph.index_of(node_id)
            }
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
            self.top_down,
            area,
            buf,
        );

        for pos in &state.layout.nodes {
            let Some(render_area) = clip_to_viewport(
                pos.x, pos.y, pos.width, pos.height, scroll_x, scroll_y, area,
            ) else {
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

        render_scrollbars(
            buf,
            area,
            state.layout.content_width,
            state.layout.content_height,
            scroll_x,
            scroll_y,
        );
    }
}
