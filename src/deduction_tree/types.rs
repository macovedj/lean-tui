use std::{error::Error, fmt};

use serde::de::DeserializeOwned;

use crate::{
    graph::{Dag, DagNode, NavNode},
    protocol::{
        AnnotatedTextTree, DisplayStyle, GenericDag, GraphNode, LineCharacterPosition,
        PreresolvedNavigationTargets,
    },
    shared::LayoutSize,
};

/// A hypothesis in a proof context.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofContextHypothesis {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: AnnotatedTextTree,
    #[serde(default)]
    pub value: Option<AnnotatedTextTree>,
    pub id: String,
    #[serde(default)]
    pub is_proof_term: bool,
    #[serde(default)]
    pub is_typeclass_instance: bool,
    #[serde(default)]
    pub is_implicit: bool,
    #[serde(default)]
    pub is_instance: bool,
    #[serde(default)]
    pub is_removed: bool,
    #[serde(default)]
    pub navigation_locations: Option<PreresolvedNavigationTargets>,
}

impl ProofContextHypothesis {
    /// Get navigation locations with default if missing.
    pub fn navigation_locations(&self) -> PreresolvedNavigationTargets {
        self.navigation_locations.clone().unwrap_or_default()
    }
}

/// A proof obligation (goal) to be proven.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofObligation {
    #[serde(rename = "type")]
    pub type_: AnnotatedTextTree,
    #[serde(default)]
    pub username: Option<String>,
    pub id: String,
    #[serde(default)]
    pub is_removed: bool,
    #[serde(default)]
    pub navigation_locations: Option<PreresolvedNavigationTargets>,
}

impl ProofObligation {
    /// Get username as string slice.
    pub fn username_str(&self) -> Option<&str> {
        self.username.as_deref()
    }
}

/// A proof state containing goals and hypotheses.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct TacticProofState {
    #[serde(default)]
    pub goals: Vec<ProofObligation>,
    #[serde(default)]
    pub hypotheses: Vec<ProofContextHypothesis>,
}

/// Type alias for node IDs.
pub type NodeId = u64;

/// Tactic text with position info.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct TacticText {
    pub text: String,
    pub position: LineCharacterPosition,
}

/// Proof DAG tactic node.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Default)]
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

    /// Convert to petgraph-based DagGraph for rendering.
    pub fn to_graph(&self) -> crate::graph::DagGraph<ProofDagTacticNode> {
        use ratatui::style::Style;

        use crate::{
            graph::{DagGraph, EdgeData},
            shared::Theme,
        };

        let mut graph = DagGraph::new();

        for node in &self.nodes {
            graph.add_node(node.id, node.clone());
        }

        for node in &self.nodes {
            let Some(from_idx) = graph.index_of(node.id) else {
                continue;
            };
            for &child_id in &node.children {
                let Some(to_idx) = graph.index_of(child_id) else {
                    continue;
                };
                graph.add_edge(
                    from_idx,
                    to_idx,
                    EdgeData {
                        style: Some(Style::new().fg(Theme::BORDER)),
                        label: None,
                    },
                );
            }
        }

        if let Some(root_id) = self.root_node_id {
            if let Some(idx) = graph.index_of(root_id) {
                graph.root = Some(idx);
            }
        }
        if let Some(current_id) = self.current_node_id {
            graph.current = graph.index_of(current_id);
        }
        for &orphan_id in &self.orphans {
            if let Some(idx) = graph.index_of(orphan_id) {
                graph.orphans.push(idx);
            }
        }

        graph
    }
}

#[derive(Debug, Clone)]
pub enum ProofDagConversionError {
    WrongDisplayStyle {
        expected: DisplayStyle,
        actual: DisplayStyle,
    },
    MetadataParse {
        node_id: u64,
        field: &'static str,
        message: String,
    },
}

impl fmt::Display for ProofDagConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongDisplayStyle { expected, actual } => {
                write!(f, "Expected display_style {expected:?}, got {actual:?}")
            }
            Self::MetadataParse {
                node_id,
                field,
                message,
            } => {
                write!(f, "Node {node_id}: failed to parse {field}: {message}")
            }
        }
    }
}

impl Error for ProofDagConversionError {}

fn parse_metadata_field<T: DeserializeOwned>(
    metadata: &serde_json::Map<String, serde_json::Value>,
    node_id: u64,
    field: &'static str,
) -> Result<Option<T>, ProofDagConversionError> {
    let Some(value) = metadata.get(field) else {
        return Ok(None);
    };
    serde_json::from_value(value.clone())
        .map(Some)
        .map_err(|e| ProofDagConversionError::MetadataParse {
            node_id,
            field,
            message: e.to_string(),
        })
}

fn parse_graph_node(node: &GraphNode) -> Result<ProofDagTacticNode, ProofDagConversionError> {
    tracing::debug!(
        "Node {}: metadata keys = {:?}",
        node.id,
        node.metadata.keys().collect::<Vec<_>>()
    );
    if let Some(ps) = node.metadata.get("proof_state_after") {
        tracing::debug!("Node {}: proof_state_after = {}", node.id, ps);
    }

    let proof_state_before: TacticProofState =
        parse_metadata_field(&node.metadata, node.id, "proof_state_before")?.unwrap_or_default();
    let proof_state_after: TacticProofState =
        parse_metadata_field(&node.metadata, node.id, "proof_state_after")?.unwrap_or_default();

    tracing::debug!(
        "Node {}: parsed {} goals, {} hypotheses",
        node.id,
        proof_state_after.goals.len(),
        proof_state_after.hypotheses.len()
    );

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

    Ok(ProofDagTacticNode {
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
    })
}

impl TryFrom<GenericDag> for CompleteProofDag {
    type Error = ProofDagConversionError;

    fn try_from(dag: GenericDag) -> Result<Self, Self::Error> {
        if dag.display_style != DisplayStyle::Proof {
            return Err(ProofDagConversionError::WrongDisplayStyle {
                expected: DisplayStyle::Proof,
                actual: dag.display_style,
            });
        }

        let nodes: Result<Vec<ProofDagTacticNode>, Self::Error> =
            dag.nodes.iter().map(parse_graph_node).collect();
        let nodes = nodes?;

        let initial_proof_state = nodes
            .first()
            .map(|n| n.proof_state_before.clone())
            .unwrap_or_default();

        Ok(Self {
            nodes,
            root_node_id: dag.root_node_id,
            current_node_id: dag.current_node_id,
            orphans: dag.orphans,
            initial_proof_state,
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

impl DagNode for ProofDagTacticNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn children(&self) -> Vec<NodeId> {
        self.children.clone()
    }
}

impl LayoutSize for ProofDagTacticNode {
    fn layout_size(&self) -> (u16, u16) {
        let width = {
            let title_width = self.tactic.text.len() + 8;
            (title_width + 4) as u16
        };
        let height = {
            // Border (2) + goals line (1) + optional hypotheses line (1)
            let has_new_hyps = !self.new_hypothesis_indices.is_empty();
            let content_lines = if has_new_hyps { 2 } else { 1 };
            2 + content_lines
        };
        (width, height)
    }
}

impl Dag for CompleteProofDag {
    type Node = ProofDagTacticNode;

    fn get_node(&self, id: NodeId) -> Option<&Self::Node> {
        self.get(id)
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
