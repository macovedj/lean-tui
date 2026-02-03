//! Protocol types generated from lean-dag JSON schema.
//!
//! Types are generated at build time from `protocol-schema.json`.
//! This module re-exports them and adds helper methods.

// Include generated types (suppress lints for generated code)
#[allow(
    clippy::absolute_paths,
    clippy::clone_on_copy,
    clippy::default_trait_access,
    clippy::derivable_impls,
    clippy::derive_partial_eq_without_eq,
    clippy::doc_markdown,
    clippy::use_self,
    dead_code
)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/protocol_types.rs"));
}
pub use generated::*;

/// Type alias for node IDs.
pub type NodeId = u64;

impl ServerToClientMessage {
    /// Get a string describing the message type for logging.
    pub const fn message_type(&self) -> &'static str {
        match self {
            Self::Connected { .. } => "Connected",
            Self::Cursor { .. } => "Cursor",
            Self::Dag { .. } => "Dag",
            Self::Error { .. } => "Error",
        }
    }
}

impl Default for AnnotatedTextTree {
    fn default() -> Self {
        Self::Text {
            text: String::new(),
        }
    }
}

impl AnnotatedTextTree {
    /// Extract plain text from the tagged structure (flattens all tags).
    pub fn to_plain_text(&self) -> String {
        match self {
            Self::Text { text } => text.clone(),
            Self::Tag { tagged_content, .. } => tagged_content.to_plain_text(),
            Self::Append { children } => children.iter().map(Self::to_plain_text).collect(),
        }
    }

    /// Check if any sub expression has a diff status set.
    pub fn has_any_diff(&self) -> bool {
        match self {
            Self::Text { .. } => false,
            Self::Tag {
                subexpression_info,
                tagged_content,
            } => subexpression_info.diff_status.is_some() || tagged_content.has_any_diff(),
            Self::Append { children } => children.iter().any(Self::has_any_diff),
        }
    }
}

impl GenericDag {
    /// Get a node by ID.
    pub fn get(&self, id: NodeId) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }
}
impl EditorCursorPosition {
    /// Extract filename from URI.
    pub fn filename(&self) -> Option<&str> {
        self.uri.rsplit('/').next()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for LineCharacterPosition {
    fn default() -> Self {
        Self {
            line: 0,
            character: 0,
        }
    }
}

impl Copy for LineCharacterPosition {}

/// Trait for DAG node types that can be lain out.
pub trait DagNode {
    fn id(&self) -> NodeId;
    fn children(&self) -> Vec<NodeId>;
}

/// Trait for DAG containers that can be lain out.
pub trait Dag {
    /// The node type stored in this DAG.
    type Node: DagNode;
    /// Get a node by ID.
    fn get_node(&self, id: NodeId) -> Option<&Self::Node>;
    /// Get the root node ID if present.
    fn root_id(&self) -> Option<NodeId>;
    /// Get orphan node IDs.
    fn orphan_ids(&self) -> &[NodeId];
}

impl DagNode for GraphNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn children(&self) -> Vec<NodeId> {
        self.edges.iter().map(|e| e.target).collect()
    }
}

impl Dag for GenericDag {
    type Node = GraphNode;

    fn get_node(&self, id: NodeId) -> Option<&Self::Node> {
        self.get(id)
    }

    fn root_id(&self) -> Option<NodeId> {
        self.root_node_id
    }

    fn orphan_ids(&self) -> &[NodeId] {
        &self.orphans
    }
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

impl NavNode for GraphNode {
    fn new_hyp_indices(&self) -> Vec<u64> {
        self.metadata
            .get("new_binding_indices")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    fn goal_count(&self) -> usize {
        usize::from(self.edges.is_empty())
    }

    fn has_selectable_goals(&self) -> bool {
        self.edges.is_empty()
    }
}
