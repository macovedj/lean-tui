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
            Self::ProofDag { .. } => "ProofDag",
            Self::FunctionalDag { .. } => "FunctionalDag",
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

    /// Get initial proof state.
    pub fn initial_proof_state(&self) -> &TacticProofState {
        &self.initial_proof_state
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

// --- Functional DAG helpers ---

impl CompleteFunctionalDag {
    /// Get a node by ID.
    pub fn get(&self, id: NodeId) -> Option<&FunctionalDagNode> {
        self.nodes.get(id as usize)
    }

    /// Iterate nodes in depth-first order.
    pub fn dfs_iter(&self) -> impl Iterator<Item = &FunctionalDagNode> {
        FunctionalDfsIterator {
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
}

/// Depth-first iterator over functional DAG nodes.
struct FunctionalDfsIterator<'a> {
    dag: &'a CompleteFunctionalDag,
    stack: Vec<NodeId>,
}

impl<'a> Iterator for FunctionalDfsIterator<'a> {
    type Item = &'a FunctionalDagNode;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.dag.get(id)?;
        for &child_id in node.children.iter().rev() {
            self.stack.push(child_id);
        }
        Some(node)
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

// --- Generic DAG traits for layout calculation ---

/// Trait for DAG node types that can be laid out.
pub trait DagNode {
    /// Get the node's unique ID.
    fn id(&self) -> NodeId;
    /// Get the node's children IDs.
    fn children(&self) -> &[NodeId];
    /// Calculate box height based on content.
    fn box_height(&self) -> u16;
    /// Calculate minimum width needed for content.
    fn content_width(&self) -> u16;
}

/// Trait for DAG containers that can be laid out.
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

impl DagNode for ProofDagTacticNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn children(&self) -> &[NodeId] {
        &self.children
    }

    fn box_height(&self) -> u16 {
        // Border (2) + goals line (1) + optional hypotheses line (1)
        3 + u16::from(!self.new_hypothesis_indices.is_empty())
    }

    fn content_width(&self) -> u16 {
        use std::iter::once;

        let title_width = self.tactic.text.len() + 8;

        let hypothesis_widths = self.new_hypothesis_indices.iter().filter_map(|&idx| {
            self.proof_state_after
                .hypotheses
                .get(idx as usize)
                .map(|h| h.name.len() + h.type_.to_plain_text().len() + 5)
        });

        let goal_widths = if self.proof_state_after.goals.is_empty() {
            vec![16] // "✓ Goal completed"
        } else {
            self.proof_state_after
                .goals
                .iter()
                .map(|g| g.type_.to_plain_text().len() + 4)
                .collect()
        };

        let max_width = once(title_width)
            .chain(hypothesis_widths)
            .chain(goal_widths)
            .max()
            .unwrap_or(0);

        (max_width + 4) as u16
    }
}

impl Dag for CompleteProofDag {
    type Node = ProofDagTacticNode;

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

impl DagNode for FunctionalDagNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn children(&self) -> &[NodeId] {
        &self.children
    }

    fn box_height(&self) -> u16 {
        // Border (2) + type line (1) + optional bindings line (1)
        3 + u16::from(!self.new_binding_indices.is_empty())
    }

    fn content_width(&self) -> u16 {
        use std::iter::once;

        let title_width = self.expression.to_plain_text().len() + 8;

        let binding_widths = self.new_binding_indices.iter().filter_map(|&idx| {
            self.bindings_after
                .get(idx as usize)
                .map(|b| b.name.len() + b.type_.to_plain_text().len() + 5)
        });

        let type_width = self
            .expected_type
            .as_ref()
            .map_or(16, |t| t.to_plain_text().len() + 4); // "✓ Complete"

        let max_width = once(title_width)
            .chain(binding_widths)
            .chain(once(type_width))
            .max()
            .unwrap_or(0);

        (max_width + 4) as u16
    }
}

impl Dag for CompleteFunctionalDag {
    type Node = FunctionalDagNode;

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

// --- Navigation traits for building clickable regions ---

/// Trait for nodes that can have navigable hypothesis/binding regions.
pub trait NavNode: DagNode {
    /// Indices of new hypotheses/bindings introduced by this node.
    fn new_hyp_indices(&self) -> &[u64];
    /// Number of goals (0 = complete, 1+ = has goals).
    fn goal_count(&self) -> usize;
    /// Whether to show goal regions (false if internal node without expected
    /// type).
    fn has_selectable_goals(&self) -> bool;
}

impl NavNode for ProofDagTacticNode {
    fn new_hyp_indices(&self) -> &[u64] {
        &self.new_hypothesis_indices
    }

    fn goal_count(&self) -> usize {
        self.proof_state_after.goals.len()
    }

    fn has_selectable_goals(&self) -> bool {
        !self.proof_state_after.goals.is_empty()
    }
}

impl NavNode for FunctionalDagNode {
    fn new_hyp_indices(&self) -> &[u64] {
        &self.new_binding_indices
    }

    fn goal_count(&self) -> usize {
        // Functional nodes have at most one "goal" (expected type)
        usize::from(self.expected_type.is_some() || self.children.is_empty())
    }

    fn has_selectable_goals(&self) -> bool {
        // Show goal region if has expected_type OR is a leaf node
        self.expected_type.is_some() || self.children.is_empty()
    }
}
