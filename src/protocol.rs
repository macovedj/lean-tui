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
pub type NodeId = i64;

impl ServerToClientMessage {
    /// Get a string describing the message type for logging.
    pub const fn message_type(&self) -> &'static str {
        match self {
            Self::Connected { .. } => "Connected",
            Self::Cursor { .. } => "Cursor",
            Self::ProofDag { .. } => "ProofDag",
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

impl CompleteProofDag {
    /// Get a node by ID.
    pub fn get(&self, id: NodeId) -> Option<&ProofDagTacticNode> {
        self.nodes.get(id as usize)
    }

    /// Iterate nodes in depth-first order.
    pub fn dfs_iter(&self) -> impl Iterator<Item = &ProofDagTacticNode> {
        DfsIterator {
            dag: self,
            stack: self.root_node_id.into_iter().collect(),
        }
    }

    /// Check if the DAG is empty.
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if a node is the current node (closest to cursor).
    pub fn is_current(&self, node_id: NodeId) -> bool {
        self.current_node_id == Some(node_id)
    }

    /// Get initial state with default if missing.
    pub fn initial_proof_state(&self) -> TacticProofState {
        self.initial_proof_state.clone().unwrap_or_default()
    }
}

impl ProofDagTacticNode {
    /// All sub-goals solved (`goals_after` empty).
    pub fn is_complete(&self) -> bool {
        self.proof_state_after.goals.is_empty()
    }

    /// No children in tree.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

impl ProofObligation {
    /// Get username as string slice (compatibility helper).
    pub fn username_str(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Get navigation locations with default if missing.
    #[allow(dead_code)]
    pub fn navigation_locations(&self) -> PreresolvedNavigationTargets {
        self.navigation_locations.clone().unwrap_or_default()
    }
}

impl ProofContextHypothesis {
    /// Get navigation locations with default if missing.
    pub fn navigation_locations(&self) -> PreresolvedNavigationTargets {
        self.navigation_locations.clone().unwrap_or_default()
    }
}

impl EditorCursorPosition {
    /// Extract filename from URI.
    pub fn filename(&self) -> Option<&str> {
        self.uri.rsplit('/').next()
    }
}

// LineCharacterPosition's generated derive doesn't include Default, so
// implement manually
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

/// Depth-first iterator over proof DAG nodes.
struct DfsIterator<'a> {
    dag: &'a CompleteProofDag,
    stack: Vec<NodeId>,
}

impl<'a> Iterator for DfsIterator<'a> {
    type Item = &'a ProofDagTacticNode;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.dag.get(id)?;
        for &child_id in node.children.iter().rev() {
            self.stack.push(child_id);
        }
        Some(node)
    }
}
