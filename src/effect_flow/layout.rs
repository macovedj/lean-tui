use ratatui::{buffer::Buffer, layout::Rect, widgets::StatefulWidget};

use super::{
    graph_pane::{render_graph_pane, GraphPaneState},
    types::EffectFlowDag,
};
use crate::{
    graph_layout::LayoutConfig,
    protocol::NodeId,
    selection::{find_region_at, Region, Selection},
};

/// State for the effect flow tableau layout.
#[derive(Default)]
pub struct EffectTableauState {
    pub graph: GraphPaneState,
}

impl EffectTableauState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        find_region_at(self.graph.click_regions.as_slice(), x, y)
    }

    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        self.graph.update_current_node(current_node_id);
    }

    pub fn navigation_regions(&self) -> Vec<Region> {
        self.graph.navigation_regions().to_vec()
    }
}

/// Effect flow tableau layout widget.
pub struct EffectTableauLayout<'a> {
    dag: &'a EffectFlowDag,
    selection: Option<Selection>,
}

impl<'a> EffectTableauLayout<'a> {
    pub fn new(dag: &'a EffectFlowDag, selection: Option<Selection>) -> Self {
        Self { dag, selection }
    }
}

impl StatefulWidget for EffectTableauLayout<'_> {
    type State = EffectTableauState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        render_graph_pane(
            self.dag,
            LayoutConfig { top_down: true },
            self.selection,
            area,
            buf,
            &mut state.graph,
        );
    }
}
