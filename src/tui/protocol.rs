//! Protocol types generated from lean-dag JSON schema.
//!
//! Types are generated at build time from `protocol-schema.json`.
//! This module re-exports them and adds helper methods.

use std::env;

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

/// Default TCP port for lean-dag connection.
pub const DEFAULT_TCP_PORT: u16 = 9742;

/// Returns the TCP address for connecting to lean-dag.
pub fn tcp_address() -> String {
    let port = env::var("LEAN_DAG_TCP_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_TCP_PORT);
    format!("127.0.0.1:{port}")
}

/// Type alias for node IDs.
pub type NodeId = i64;

impl Message {
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

impl Default for TaggedText {
    fn default() -> Self {
        Self::Text {
            text: String::new(),
        }
    }
}

impl TaggedText {
    /// Extract plain text from the tagged structure (flattens all tags).
    pub fn to_plain_text(&self) -> String {
        match self {
            Self::Text { text } => text.clone(),
            Self::Tag { content, .. } => content.to_plain_text(),
            Self::Append { items } => items.iter().map(Self::to_plain_text).collect(),
        }
    }

    /// Check if any sub expression has a diff status set.
    pub fn has_any_diff(&self) -> bool {
        match self {
            Self::Text { .. } => false,
            Self::Tag { info, content } => info.diff_status.is_some() || content.has_any_diff(),
            Self::Append { items } => items.iter().any(Self::has_any_diff),
        }
    }
}

impl ProofDag {
    /// Get a node by ID.
    pub fn get(&self, id: NodeId) -> Option<&ProofDagNode> {
        self.nodes.get(id as usize)
    }

    /// Iterate nodes in depth-first order.
    pub fn dfs_iter(&self) -> impl Iterator<Item = &ProofDagNode> {
        DfsIterator {
            dag: self,
            stack: self.root.into_iter().collect(),
        }
    }

    /// Check if the DAG is empty.
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get number of nodes.
    #[allow(clippy::missing_const_for_fn)]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if a node is the current node (closest to cursor).
    pub fn is_current(&self, node_id: NodeId) -> bool {
        self.current_node == Some(node_id)
    }

    /// Get initial state with default if missing.
    pub fn initial_state(&self) -> ProofState {
        self.initial_state.clone().unwrap_or_default()
    }
}

impl ProofDagNode {
    /// All sub-goals solved (`goals_after` empty).
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_complete(&self) -> bool {
        self.state_after.goals.is_empty()
    }

    /// No children in tree.
    #[allow(clippy::missing_const_for_fn)]
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }
}

impl GoalInfo {
    /// Get username as string slice (compatibility helper).
    pub fn username_str(&self) -> Option<&str> {
        self.username.as_deref()
    }

    /// Get goto locations with default if missing.
    #[allow(dead_code)]
    pub fn goto_locations(&self) -> GotoLocations {
        self.goto_locations.clone().unwrap_or_default()
    }
}

impl HypothesisInfo {
    /// Get goto locations with default if missing.
    pub fn goto_locations(&self) -> GotoLocations {
        self.goto_locations.clone().unwrap_or_default()
    }
}

impl CursorInfo {
    /// Extract filename from URI.
    pub fn filename(&self) -> Option<&str> {
        self.uri.rsplit('/').next()
    }
}

// Position's generated derive doesn't include Default, so implement manually
#[allow(clippy::derivable_impls)]
impl Default for Position {
    fn default() -> Self {
        Self {
            line: 0,
            character: 0,
        }
    }
}

impl Copy for Position {}

/// Depth-first iterator over proof DAG nodes.
struct DfsIterator<'a> {
    dag: &'a ProofDag,
    stack: Vec<NodeId>,
}

impl<'a> Iterator for DfsIterator<'a> {
    type Item = &'a ProofDagNode;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.dag.get(id)?;
        for &child_id in node.children.iter().rev() {
            self.stack.push(child_id);
        }
        Some(node)
    }
}
