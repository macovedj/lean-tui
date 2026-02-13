use std::{collections::HashMap, error::Error, fmt};

use petgraph::graph::NodeIndex;
use ratatui::style::Style;
use serde::de::DeserializeOwned;

use crate::{
    graph_layout::{
        calculate_node_size, Dag, DagGraph, DagNode, EdgeData, LayoutGraph, LayoutSize, NavNode,
        NodeTextContent,
    },
    protocol::{
        DiagnosticInfo, DiagnosticSeverity, DisplayStyle, GenericDag, GraphNode,
        LineCharacterPosition, LocalBinding, NodeId, PreresolvedNavigationTargets, RichText,
    },
    selection::Selection,
    theme::Theme,
};

/// A hypothesis in a proof context.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProofContextHypothesis {
    pub name: String,
    #[serde(rename = "type")]
    pub type_: RichText,
    #[serde(default)]
    pub value: Option<RichText>,
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
    pub navigation_locations: Option<PreresolvedNavigationTargets>,
}

impl ProofContextHypothesis {
    /// Get navigation locations with default if missing.
    pub fn navigation_locations(&self) -> PreresolvedNavigationTargets {
        self.navigation_locations.clone().unwrap_or_default()
    }
}

/// A goal to be proven.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalState {
    #[serde(rename = "type")]
    pub type_: RichText,
    #[serde(default)]
    pub username: Option<String>,
    pub id: String,
    #[serde(default)]
    pub navigation_locations: Option<PreresolvedNavigationTargets>,
}

impl GoalState {
    /// Get username as string slice.
    pub fn username_str(&self) -> Option<&str> {
        self.username.as_deref()
    }
}

/// A proof state containing goals and hypotheses.
#[derive(Clone, Debug, Default, serde::Deserialize, serde::Serialize)]
pub struct TacticProofState {
    #[serde(default)]
    pub goals: Vec<GoalState>,
    #[serde(default)]
    pub hypotheses: Vec<ProofContextHypothesis>,
}

/// Proof DAG tactic node.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, Default)]
pub struct ProofDagTacticNode {
    pub id: NodeId,
    pub tactic_text: String,
    pub position: LineCharacterPosition,
    #[serde(default)]
    pub end_position: Option<LineCharacterPosition>,
    pub proof_state: TacticProofState,
    pub children: Vec<NodeId>,
    pub parent: Option<NodeId>,
    pub depth: u64,
    #[serde(default)]
    pub new_hypothesis_indices: Vec<u64>,
    #[serde(default)]
    pub hypothesis_dependencies: Vec<String>,
    #[serde(default)]
    pub referenced_theorems: Vec<String>,
    #[serde(default)]
    pub is_complete: bool,
    pub diagnostics: Vec<DiagnosticInfo>,
}

impl ProofDagTacticNode {
    /// No children in tree.
    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    /// Check if this node has any error diagnostics.
    pub fn has_error(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }

    /// Check if this node's source range contains the cursor position.
    pub fn contains_cursor(&self, cursor: LineCharacterPosition) -> bool {
        crate::protocol::range_contains_cursor(self.position, self.end_position, cursor)
    }

    /// Compute rough size of this node's range.
    pub fn range_size(&self) -> u64 {
        crate::protocol::range_size(self.position, self.end_position)
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
    pub block_output: Option<RichText>,
    #[serde(default)]
    pub block_input: Vec<LocalBinding>,
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

    /// Update current_node_id based on cursor position using range containment.
    /// Returns true if the current node changed.
    pub fn update_current_from_cursor(&mut self, cursor: LineCharacterPosition) -> bool {
        let old = self.current_node_id;
        self.current_node_id = self.find_node_at_cursor(cursor);
        old != self.current_node_id
    }

    /// Find the node whose range contains the cursor position.
    fn find_node_at_cursor(&self, cursor: LineCharacterPosition) -> Option<NodeId> {
        self.nodes
            .iter()
            .filter(|n| n.contains_cursor(cursor))
            .min_by_key(|n| n.range_size())
            .map(|n| n.id)
    }

    /// Get the error message at the given index for a node.
    pub fn get_error_message(&self, node_id: NodeId, error_idx: usize) -> Option<String> {
        self.get(node_id).and_then(|node| {
            node.diagnostics
                .iter()
                .filter(|d| d.severity == DiagnosticSeverity::Error)
                .nth(error_idx)
                .map(|d| d.message.clone())
        })
    }

    /// Get the text content for a given selection.
    pub fn get_selection_text(&self, selection: Selection) -> Option<String> {
        match selection {
            Selection::InitialHyp { hyp_idx } => self
                .block_input
                .get(hyp_idx)
                .map(|b| format!("{} : {}", b.name, b.type_.to_plain_text())),
            Selection::Hyp { node_id, hyp_idx } => self
                .get(node_id)
                .and_then(|node| node.proof_state.hypotheses.get(hyp_idx))
                .map(|h| format!("{} : {}", h.name, h.type_.to_plain_text())),
            Selection::Goal { node_id, goal_idx } => self
                .get(node_id)
                .and_then(|node| node.proof_state.goals.get(goal_idx))
                .map(|g| g.type_.to_plain_text()),
            Selection::Theorem => self.block_output.as_ref().map(RichText::to_plain_text),
            Selection::Error { node_id, error_idx } => self.get_error_message(node_id, error_idx),
        }
    }

    /// Get the navigation location for a selection.
    pub fn navigation_location(
        &self,
        selection: Selection,
        current_uri: &str,
    ) -> Option<(String, LineCharacterPosition)> {
        match selection {
            Selection::InitialHyp { hyp_idx } => self
                .block_input
                .get(hyp_idx)
                .and_then(|b| b.navigation_locations.as_ref())
                .and_then(|nav| nav.definition.as_ref())
                .map(|loc| (loc.uri.clone(), loc.position)),
            Selection::Hyp { node_id, hyp_idx } => self
                .get(node_id)
                .and_then(|node| node.proof_state.hypotheses.get(hyp_idx))
                .and_then(|h| h.navigation_locations().definition)
                .map(|loc| (loc.uri, loc.position)),
            Selection::Goal { node_id, .. } => self
                .get(node_id)
                .map(|node| (current_uri.to_string(), node.position)),
            Selection::Theorem | Selection::Error { .. } => None,
        }
    }

    /// Convert to petgraph-based `DagGraph` for rendering.
    pub fn to_graph(&self) -> DagGraph<ProofDagTacticNode> {
        let mut graph = DagGraph::new();

        for node in &self.nodes {
            graph.add_node(node.id, node.clone());
        }

        for node in &self.nodes {
            add_proof_edges(node, &mut graph);
        }

        graph.root = self.root_node_id.and_then(|id| graph.index_of(id));
        graph.current = self.current_node_id.and_then(|id| graph.index_of(id));
        graph.orphans = self
            .orphans
            .iter()
            .filter_map(|&id| graph.index_of(id))
            .collect();

        graph
    }

    /// Convert to lay out graph for force-directed layout computation.
    pub fn to_layout_graph(&self) -> LayoutGraph {
        let mut layout = LayoutGraph::new();
        let mut id_to_index = HashMap::new();

        for node in &self.nodes {
            let (width, height) = node.layout_size();
            let index = NodeIndex::new(layout.nodes.len());
            id_to_index.insert(node.id, index);
            layout.add_node(index, node.id, width, height);
        }

        for node in &self.nodes {
            add_proof_layout_edges(node, &id_to_index, &mut layout);
        }

        layout.root = self
            .root_node_id
            .and_then(|id| id_to_index.get(&id).copied());
        layout.orphans = self
            .orphans
            .iter()
            .filter_map(|id| id_to_index.get(id).copied())
            .collect();

        layout
    }
}

fn add_proof_edges(node: &ProofDagTacticNode, graph: &mut DagGraph<ProofDagTacticNode>) {
    let Some(from_idx) = graph.index_of(node.id) else {
        return;
    };
    for &child_id in &node.children {
        let Some(to_idx) = graph.index_of(child_id) else {
            continue;
        };
        graph.add_edge(
            from_idx,
            to_idx,
            EdgeData {
                style: Some(Style::new().fg(Theme::PANE_BORDER_FOCUSED)),
                label: None,
            },
        );
    }
}

fn add_proof_layout_edges(
    node: &ProofDagTacticNode,
    id_to_index: &HashMap<NodeId, NodeIndex>,
    layout: &mut LayoutGraph,
) {
    let Some(&from_idx) = id_to_index.get(&node.id) else {
        return;
    };
    for &child_id in &node.children {
        let Some(&to_idx) = id_to_index.get(&child_id) else {
            continue;
        };
        layout.add_edge(from_idx, to_idx, 1);
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
    if let Some(ps) = node.metadata.get("proof_state") {
        tracing::debug!("Node {}: proof_state = {}", node.id, ps);
    }
    if !node.diagnostics.is_empty() {
        tracing::info!(
            "Node {}: has {} diagnostics: {:?}",
            node.id,
            node.diagnostics.len(),
            node.diagnostics
        );
    }

    let proof_state: TacticProofState =
        parse_metadata_field(&node.metadata, node.id, "proof_state")?.unwrap_or_default();

    tracing::debug!(
        "Node {}: parsed {} goals, {} hypotheses",
        node.id,
        proof_state.goals.len(),
        proof_state.hypotheses.len()
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

    let is_complete: bool = node
        .metadata
        .get("is_complete")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);

    let children: Vec<NodeId> = node.edges.iter().map(|e| e.target).collect();

    Ok(ProofDagTacticNode {
        id: node.id,
        tactic_text: node.content.to_plain_text(),
        position: node.position,
        end_position: node.end_position,
        proof_state,
        children,
        parent: node.parent,
        depth: node.depth,
        new_hypothesis_indices,
        hypothesis_dependencies,
        referenced_theorems,
        is_complete,
        diagnostics: node.diagnostics.clone(),
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

        Ok(Self {
            nodes,
            root_node_id: dag.root_node_id,
            current_node_id: dag.current_node_id,
            orphans: dag.orphans,
            block_output: dag.block_output,
            block_input: dag.block_input,
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

impl NodeTextContent for ProofDagTacticNode {
    fn title(&self) -> String {
        self.tactic_text.clone()
    }

    fn content_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        for h in &self.proof_state.hypotheses {
            lines.push(format!("{} : {}", h.name, h.type_.to_plain_text()));
        }

        if self.proof_state.goals.is_empty() {
            lines.push("✓ Goal completed".to_string());
        } else {
            for g in &self.proof_state.goals {
                let goal_type = g.type_.to_plain_text();
                lines.push(g.username_str().map_or_else(
                    || format!("⊢ {goal_type}"),
                    |name| format!("{name} ⊢ {goal_type}"),
                ));
            }
        }

        for d in &self.diagnostics {
            if d.severity == DiagnosticSeverity::Error {
                let msg = d.message.lines().next().unwrap_or(&d.message);
                lines.push(msg.to_string());
            }
        }

        lines
    }
}

impl LayoutSize for ProofDagTacticNode {
    fn layout_size(&self) -> (u16, u16) {
        calculate_node_size(self)
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
        self.proof_state.goals.len()
    }

    fn has_selectable_goals(&self) -> bool {
        !self.proof_state.goals.is_empty()
    }
}
