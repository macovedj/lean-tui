use std::collections::HashMap;

use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
    Direction,
};
use ratatui::style::Style;

use crate::protocol::NodeId;

/// Trait for DAG node types that can be laid out.
pub trait DagNode {
    fn id(&self) -> NodeId;
    fn children(&self) -> Vec<NodeId>;
}

/// Trait for DAG containers that can be laid out.
pub trait Dag {
    /// The node type stored in this DAG.
    type Node: DagNode;
    /// Get a node by ID.
    fn get_node(&self, id: NodeId) -> Option<&Self::Node>;
}

/// Trait for nodes that can have navigable hypothesis/binding regions.
pub trait NavNode: DagNode {
    /// Number of hypotheses/bindings in this node.
    fn hyp_count(&self) -> usize;
    /// Number of goals (0 = complete, 1+ = has goals).
    fn goal_count(&self) -> usize;
    /// Whether to show goal regions.
    fn has_selectable_goals(&self) -> bool;
}

#[derive(Debug, Clone, Default)]
pub struct EdgeData {
    pub style: Option<Style>,
    pub label: Option<String>,
}

pub struct DagGraph<N> {
    pub graph: DiGraph<N, EdgeData>,
    pub id_to_index: HashMap<NodeId, NodeIndex>,
    pub root: Option<NodeIndex>,
    pub current: Option<NodeIndex>,
    pub orphans: Vec<NodeIndex>,
}

impl<N> DagGraph<N> {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            id_to_index: HashMap::new(),
            root: None,
            current: None,
            orphans: Vec::new(),
        }
    }

    pub fn add_node(&mut self, id: NodeId, data: N) -> NodeIndex {
        let idx = self.graph.add_node(data);
        self.id_to_index.insert(id, idx);
        idx
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, data: EdgeData) {
        self.graph.add_edge(from, to, data);
    }

    pub fn get(&self, idx: NodeIndex) -> Option<&N> {
        self.graph.node_weight(idx)
    }

    pub fn index_of(&self, id: NodeId) -> Option<NodeIndex> {
        self.id_to_index.get(&id).copied()
    }

    pub fn is_current(&self, idx: NodeIndex) -> bool {
        self.current == Some(idx)
    }

    pub fn edges_from(&self, idx: NodeIndex) -> impl Iterator<Item = (NodeIndex, &EdgeData)> + '_ {
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .map(|e| (e.target(), e.weight()))
    }
}

impl<N> Default for DagGraph<N> {
    fn default() -> Self {
        Self::new()
    }
}
