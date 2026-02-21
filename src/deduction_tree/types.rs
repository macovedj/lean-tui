use std::{collections::HashMap, error::Error, fmt};

use petgraph::graph::NodeIndex;
use ratatui::style::{Color, Modifier, Style};
use serde::de::DeserializeOwned;

use crate::{
    graph_layout::{Dag, DagGraph, DagNode, EdgeData, LayoutGraph, NavNode},
    protocol::{
        DiagnosticInfo, DiagnosticSeverity, DisplayStyle, GenericDag, GraphNode,
        LineCharacterPosition, LocalBinding, NodeId, PreresolvedNavigationTargets, RichText,
        range_contains_cursor, range_size,
    },
    selection::Selection,
    theme::{Red, Theme},
    widgets::{
        aligned_table::AlignedRow,
        diff_text::RichTextExt,
        node_content::{ContentItem, ErrorLine, NodeContent, NodeStyle},
    },
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
        range_contains_cursor(self.position, self.end_position, cursor)
    }

    /// Compute rough size of this node's range.
    pub fn range_size(&self) -> u64 {
        range_size(self.position, self.end_position)
    }

    pub fn build_content(
        &self,
        selection: Option<Selection>,
        is_current: bool,
    ) -> NodeContent {
        let error_prefix = if self.has_error() { "⚠ " } else { "" };
        let title = format!(" {error_prefix}{} ", self.tactic_text);

        let mut items: Vec<ContentItem> = self
            .proof_state
            .hypotheses
            .iter()
            .enumerate()
            .map(|(hyp_idx, h)| {
                let selected = matches!(
                    selection,
                    Some(Selection::Hyp { node_id, hyp_idx: hi }) if node_id == self.id && hi == hyp_idx
                );
                let underline = if selected {
                    Modifier::UNDERLINED
                } else {
                    Modifier::empty()
                };
                let name_style = Style::new().fg(Theme::HYP_NAME).add_modifier(underline);
                let type_style = Style::new().fg(Color::White).add_modifier(underline);
                ContentItem::Row {
                    row: AlignedRow::with_styled_type(
                        h.name.clone(),
                        name_style,
                        ":",
                        Style::new().fg(Color::DarkGray),
                        h.type_.to_spans(type_style),
                    ),
                    selection: Some(Selection::Hyp {
                        node_id: self.id,
                        hyp_idx,
                    }),
                }
            })
            .collect();

        items.extend(self.build_goal_items(selection, is_current));

        let errors = self.build_error_items();

        NodeContent {
            title,
            items,
            errors,
        }
    }

    fn build_goal_items(
        &self,
        selection: Option<Selection>,
        _is_current: bool,
    ) -> Vec<ContentItem> {
        if self.is_complete {
            let completed_style = Style::new()
                .fg(Theme::COMPLETED_GOAL_FG)
                .add_modifier(Modifier::BOLD);
            return vec![ContentItem::Row {
                row: AlignedRow::new(
                    "",
                    Style::default(),
                    "✓",
                    completed_style,
                    "Goal completed",
                    completed_style,
                ),
                selection: None,
            }];
        }

        if self.proof_state.goals.is_empty() {
            return vec![ContentItem::Row {
                row: AlignedRow::new(
                    "",
                    Style::default(),
                    "⊢",
                    Style::new().fg(Color::DarkGray),
                    "...",
                    Style::new().fg(Theme::GOAL),
                ),
                selection: None,
            }];
        }

        self.proof_state
            .goals
            .iter()
            .enumerate()
            .map(|(goal_idx, g)| {
                let selected = matches!(
                    selection,
                    Some(Selection::Goal { node_id, goal_idx: gi }) if node_id == self.id && gi == goal_idx
                );
                let underline = if selected {
                    Modifier::UNDERLINED
                } else {
                    Modifier::empty()
                };
                let name_text = g.username_str().unwrap_or_default().to_string();
                let name_style = if g.username_str().is_some() {
                    Style::new().fg(Theme::GOAL).add_modifier(underline)
                } else {
                    Style::new()
                };
                let type_style = Style::new().fg(Color::White).add_modifier(underline);
                ContentItem::Row {
                    row: AlignedRow::with_styled_type(
                        name_text,
                        name_style,
                        "⊢",
                        Style::new().fg(Color::DarkGray),
                        g.type_.to_spans(type_style),
                    ),
                    selection: Some(Selection::Goal {
                        node_id: self.id,
                        goal_idx,
                    }),
                }
            })
            .collect()
    }

    fn build_error_items(&self) -> Vec<ErrorLine> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .enumerate()
            .map(|(error_idx, d)| {
                let msg = d.message.lines().next().unwrap_or_default().to_string();
                ErrorLine {
                    text: msg,
                    style: Style::new().fg(Red::BRIGHT),
                    selection: Some(Selection::Error {
                        node_id: self.id,
                        error_idx,
                    }),
                }
            })
            .collect()
    }

    pub fn build_node_style(&self, is_current: bool) -> NodeStyle {
        let border_color = if self.has_error() {
            Red::BRIGHT
        } else {
            match (is_current, self.is_leaf(), self.is_complete) {
                (true, _, _) => Theme::CURRENT_NODE_BORDER,
                (_, true, true) => Theme::COMPLETED_NODE_BORDER,
                (_, true, false) => Theme::INCOMPLETE_NODE_BORDER,
                (_, false, _) => Theme::INACTIVE_NODE_BORDER,
            }
        };

        let title_color = if self.has_error() {
            Red::BRIGHT
        } else if is_current {
            Theme::CURRENT_NODE_BORDER
        } else {
            Color::Gray
        };

        NodeStyle {
            border_color,
            border_bold: is_current,
            title_color,
            title_bold: is_current || self.has_error(),
        }
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

    /// Convert to lay out graph for layered layout computation.
    /// Each node is sized based on its own proof state.
    pub fn to_layout_graph(&self) -> LayoutGraph {
        let mut layout = LayoutGraph::new();
        let mut id_to_index = HashMap::new();

        for node in &self.nodes {
            let (width, height) = node.build_content(None, false).layout_size();
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

impl Dag for CompleteProofDag {
    type Node = ProofDagTacticNode;

    fn get_node(&self, id: NodeId) -> Option<&Self::Node> {
        self.get(id)
    }
}

impl NavNode for ProofDagTacticNode {
    fn hyp_count(&self) -> usize {
        self.proof_state.hypotheses.len()
    }

    fn goal_count(&self) -> usize {
        self.proof_state.goals.len()
    }

    fn has_selectable_goals(&self) -> bool {
        !self.proof_state.goals.is_empty()
    }
}
