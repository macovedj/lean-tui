use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::StatefulWidget,
};

use super::types::ProofDagTacticNode;
use crate::{
    selection::Selection,
    widgets::node_content::{NodeContentWidget, NodeWidgetState},
};

pub type StateNodeState = NodeWidgetState;

pub struct StateNode<'a> {
    node: &'a ProofDagTacticNode,
    is_current: bool,
    selection: Option<Selection>,
}

impl<'a> StateNode<'a> {
    pub const fn new(
        node: &'a ProofDagTacticNode,
        is_current: bool,
        selection: Option<Selection>,
    ) -> Self {
        Self {
            node,
            is_current,
            selection,
        }
    }
}

impl StatefulWidget for StateNode<'_> {
    type State = StateNodeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let content = self.node.build_content(self.selection, self.is_current);
        let style = self.node.build_node_style(self.is_current);
        let widget = NodeContentWidget { content, style };
        widget.render(area, buf, state);
    }
}
