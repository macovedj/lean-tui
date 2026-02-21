use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::StatefulWidget,
};

use super::types::EffectFlowNode;
use crate::{
    selection::Selection,
    widgets::node_content::{NodeContentWidget, NodeWidgetState},
};

pub type EffectStateNodeState = NodeWidgetState;

pub struct EffectStateNode<'a> {
    node: &'a EffectFlowNode,
    is_current: bool,
    selection: Option<Selection>,
}

impl<'a> EffectStateNode<'a> {
    pub const fn new(
        node: &'a EffectFlowNode,
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

impl StatefulWidget for EffectStateNode<'_> {
    type State = EffectStateNodeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let content = self.node.build_content(self.selection, self.is_current);
        let style = self.node.build_node_style(self.is_current);
        let widget = NodeContentWidget { content, style };
        widget.render(area, buf, state);
    }
}
