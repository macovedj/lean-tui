use crate::protocol::{AnnotatedTextTree, GenericDag, GraphNode, LocalBinding};

pub trait DataFlowDagExt {
    /// Get bindings from the initial context, filtering implicit and instance
    /// bindings.
    fn visible_bindings(&self) -> impl Iterator<Item = (usize, &LocalBinding)>;
}

impl DataFlowDagExt for GenericDag {
    fn visible_bindings(&self) -> impl Iterator<Item = (usize, &LocalBinding)> {
        self.initial_context
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.is_implicit && !b.is_instance)
    }
}

pub trait DataFlowNodeExt {
    fn bindings_after(&self) -> Vec<LocalBinding>;

    fn new_binding_indices(&self) -> Vec<u64>;

    fn expected_type(&self) -> Option<AnnotatedTextTree>;

    /// Check if this is a leaf node (no children).
    fn is_leaf(&self) -> bool;

    /// Check if this node is complete.
    fn is_complete(&self) -> bool;
}

impl DataFlowNodeExt for GraphNode {
    fn bindings_after(&self) -> Vec<LocalBinding> {
        self.metadata
            .get("bindings_after")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    fn new_binding_indices(&self) -> Vec<u64> {
        self.metadata
            .get("new_binding_indices")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    fn expected_type(&self) -> Option<AnnotatedTextTree> {
        self.metadata
            .get("expected_type")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    fn is_leaf(&self) -> bool {
        self.edges.is_empty()
    }

    fn is_complete(&self) -> bool {
        self.expected_type().is_none() && self.is_leaf()
    }
}
