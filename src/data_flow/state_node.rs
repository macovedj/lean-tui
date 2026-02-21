use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::StatefulWidget,
};

use super::types::DataFlowNode;
use crate::{
    selection::Selection,
    widgets::node_content::{NodeContentWidget, NodeWidgetState},
};

pub type FunctionalStateNodeState = NodeWidgetState;

pub struct FunctionalStateNode<'a> {
    node: &'a DataFlowNode,
    is_current: bool,
    selection: Option<Selection>,
    top_down: bool,
}

impl<'a> FunctionalStateNode<'a> {
    pub const fn new(
        node: &'a DataFlowNode,
        is_current: bool,
        selection: Option<Selection>,
        top_down: bool,
    ) -> Self {
        Self {
            node,
            is_current,
            selection,
            top_down,
        }
    }
}

impl StatefulWidget for FunctionalStateNode<'_> {
    type State = FunctionalStateNodeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let content = self.node.build_content(self.selection, self.is_current, self.top_down);
        let style = self.node.build_node_style(self.is_current);
        let widget = NodeContentWidget { content, style };
        widget.render(area, buf, state);
    }
}
