//! Deduction tree types.
//!
//! These types represent the proof DAG structure used by deduction tree
//! visualization. They can be reconstructed from GenericDag with display_style
//! = "proof".

use crate::protocol::{
    AnnotatedTextTree, Dag, DagNode, DisplayStyle, GenericDag, LineCharacterPosition, NavNode,
    TacticProofState,
};

/// Type alias for node IDs.
pub type NodeId = u64;

/// Tactic text with position info.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct TacticText {
    pub text: String,
    pub position: LineCharacterPosition,
}

/// Proof DAG tactic node.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ProofDagTacticNode {
    pub id: NodeId,
    pub tactic: TacticText,
    pub position: LineCharacterPosition,
    pub proof_state_before: TacticProofState,
    pub proof_state_after: TacticProofState,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
    pub depth: u64,
    #[serde(default)]
    pub new_hypothesis_indices: Vec<u64>,
    #[serde(default)]
    pub hypothesis_dependencies: Vec<String>,
    #[serde(default)]
    pub referenced_theorems: Vec<String>,
}

impl Default for ProofDagTacticNode {
    fn default() -> Self {
        Self {
            id: 0,
            tactic: TacticText::default(),
            position: LineCharacterPosition::default(),
            proof_state_before: TacticProofState::default(),
            proof_state_after: TacticProofState::default(),
            children: Vec::new(),
            parent: None,
            depth: 0,
            new_hypothesis_indices: Vec::new(),
            hypothesis_dependencies: Vec::new(),
            referenced_theorems: Vec::new(),
        }
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

/// Complete proof DAG.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct CompleteProofDag {
    pub nodes: Vec<ProofDagTacticNode>,
    pub root_node_id: Option<NodeId>,
    pub current_node_id: Option<NodeId>,
    #[serde(default)]
    pub orphans: Vec<NodeId>,
    #[serde(default)]
    pub initial_proof_state: TacticProofState,
    pub definition_name: Option<String>,
    pub definition_type: Option<AnnotatedTextTree>,
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

/// Error when converting from GenericDag to CompleteProofDag.
#[derive(Debug, Clone)]
pub struct ProofDagConversionError {
    pub expected_style: DisplayStyle,
    pub actual_style: DisplayStyle,
}

impl std::fmt::Display for ProofDagConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Expected display_style {:?}, got {:?}",
            self.expected_style, self.actual_style
        )
    }
}

impl std::error::Error for ProofDagConversionError {}

impl TryFrom<GenericDag> for CompleteProofDag {
    type Error = ProofDagConversionError;

    fn try_from(dag: GenericDag) -> Result<Self, Self::Error> {
        if dag.display_style != DisplayStyle::Proof {
            return Err(ProofDagConversionError {
                expected_style: DisplayStyle::Proof,
                actual_style: dag.display_style,
            });
        }

        let nodes: Vec<ProofDagTacticNode> = dag
            .nodes
            .iter()
            .map(|node| {
                let proof_state_before: TacticProofState = node
                    .metadata
                    .get("proof_state_before")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let proof_state_after: TacticProofState = node
                    .metadata
                    .get("proof_state_after")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let new_hypothesis_indices: Vec<u64> = node
                    .metadata
                    .get("new_hypothesis_indices")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let hypothesis_dependencies: Vec<String> = node
                    .metadata
                    .get("hypothesis_dependencies")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let referenced_theorems: Vec<String> = node
                    .metadata
                    .get("referenced_theorems")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let children: Vec<NodeId> = node.edges.iter().map(|e| e.target).collect();

                ProofDagTacticNode {
                    id: node.id,
                    tactic: TacticText {
                        text: node.content.to_plain_text(),
                        position: node.position,
                    },
                    position: node.position,
                    proof_state_before,
                    proof_state_after,
                    children,
                    parent: node.parent,
                    depth: node.depth,
                    new_hypothesis_indices,
                    hypothesis_dependencies,
                    referenced_theorems,
                }
            })
            .collect();

        let initial_proof_state = nodes
            .first()
            .map(|n| n.proof_state_before.clone())
            .unwrap_or_default();

        Ok(CompleteProofDag {
            nodes,
            root_node_id: dag.root_node_id,
            current_node_id: dag.current_node_id,
            orphans: dag.orphans,
            initial_proof_state,
            definition_name: dag.definition_name,
            definition_type: dag.definition_type,
        })
    }
}

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

// --- Layout trait implementations ---

impl DagNode for ProofDagTacticNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn children(&self) -> Vec<NodeId> {
        self.children.clone()
    }

    fn box_height(&self) -> u16 {
        // Border (2) + content line (1)
        3
    }

    fn content_width(&self) -> u16 {
        let title_width = self.tactic.text.len() + 8;
        (title_width + 4) as u16
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

impl NavNode for ProofDagTacticNode {
    fn new_hyp_indices(&self) -> Vec<u64> {
        self.new_hypothesis_indices.clone()
    }

    fn goal_count(&self) -> usize {
        self.proof_state_after.goals.len()
    }

    fn has_selectable_goals(&self) -> bool {
        !self.proof_state_after.goals.is_empty()
    }
}
