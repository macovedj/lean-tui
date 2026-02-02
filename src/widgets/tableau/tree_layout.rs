//! Shared tree layout types for tableau views.

use crate::protocol::NodeId;

pub const MIN_NODE_WIDTH: u16 = 25;
pub const MAX_NODE_WIDTH: u16 = 60;
/// Gap between main tree and orphan nodes.
pub const ORPHAN_GAP: i32 = 4;

#[derive(Debug, Clone, Copy)]
pub struct NodePosition {
    pub node_id: NodeId,
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Default)]
pub struct TreeLayout {
    pub nodes: Vec<NodePosition>,
    pub content_width: i32,
    pub content_height: i32,
}

impl TreeLayout {
    pub fn find_node(&self, node_id: NodeId) -> Option<&NodePosition> {
        self.nodes.iter().find(|n| n.node_id == node_id)
    }
}
