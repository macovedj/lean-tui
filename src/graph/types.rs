use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
    Direction,
};
use ratatui::style::Style;
use std::collections::HashMap;

pub type NodeId = u64;

#[derive(Debug, Clone, Default)]
pub struct EdgeData {
    pub style: Option<Style>,
    pub label: Option<&'static str>,
}

pub struct DagGraph<N> {
    pub graph: DiGraph<N, EdgeData>,
    pub id_to_index: HashMap<NodeId, NodeIndex>,
    pub index_to_id: HashMap<NodeIndex, NodeId>,
    pub root: Option<NodeIndex>,
    pub current: Option<NodeIndex>,
    pub orphans: Vec<NodeIndex>,
}

impl<N> DagGraph<N> {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            id_to_index: HashMap::new(),
            index_to_id: HashMap::new(),
            root: None,
            current: None,
            orphans: Vec::new(),
        }
    }

    pub fn add_node(&mut self, id: NodeId, data: N) -> NodeIndex {
        let idx = self.graph.add_node(data);
        self.id_to_index.insert(id, idx);
        self.index_to_id.insert(idx, id);
        idx
    }

    pub fn node_id(&self, idx: NodeIndex) -> Option<NodeId> {
        self.index_to_id.get(&idx).copied()
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

    pub fn children(&self, idx: NodeIndex) -> Vec<NodeIndex> {
        self.graph
            .edges_directed(idx, Direction::Outgoing)
            .map(|e| e.target())
            .collect()
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
