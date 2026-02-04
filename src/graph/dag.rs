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
    /// Indices of new hypotheses/bindings introduced by this node.
    fn new_hyp_indices(&self) -> Vec<u64>;
    /// Number of goals (0 = complete, 1+ = has goals).
    fn goal_count(&self) -> usize;
    /// Whether to show goal regions.
    fn has_selectable_goals(&self) -> bool;
}
