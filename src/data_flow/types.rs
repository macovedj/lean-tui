use std::{collections::HashMap, error::Error, fmt, num::NonZero, ops::Deref};

use petgraph::graph::NodeIndex;
use ratatui::style::{Color, Style};

use crate::{
    graph_layout::{
        calculate_node_size, Dag, DagGraph, DagNode, EdgeData, LayoutGraph, LayoutSize, NavNode,
        NodeTextContent,
    },
    protocol::{
        BindingKind, DiagnosticInfo, DiagnosticSeverity, DisplayStyle, GenericDag, GraphNode,
        LineCharacterPosition, LocalBinding, NodeId, PreresolvedNavigationTargets, RichText,
    },
    selection::Selection,
    theme::Theme,
};

/// Data flow node kinds from the lean-prism backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataFlowKind {
    LetBinding,
    Lambda,
    Branch,
    Condition,
    ConditionPart,
    MatchScrutinee,
    Result,
    Custom(String),
}

impl DataFlowKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "binding" => Self::LetBinding,
            "lambda" => Self::Lambda,
            "branch" => Self::Branch,
            "condition" => Self::Condition,
            "conditionPart" => Self::ConditionPart,
            "matchScrutinee" => Self::MatchScrutinee,
            "result" => Self::Result,
            other => Self::Custom(other.to_owned()),
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::LetBinding => "≔",
            Self::Lambda => "λ",
            Self::Branch => "⎇",
            Self::Condition => "?",
            Self::ConditionPart => "∧",
            Self::MatchScrutinee => "⊳",
            Self::Result => "●",
            Self::Custom(_) => "◆",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::LetBinding => Theme::DATAFLOW_BINDING,
            Self::Lambda => Theme::DATAFLOW_LAMBDA,
            Self::Branch | Self::MatchScrutinee => Theme::DATAFLOW_BRANCH,
            Self::Condition | Self::ConditionPart => Theme::DATAFLOW_CONDITION,
            Self::Result => Color::Green,
            Self::Custom(_) => Color::Gray,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::LetBinding => "binding",
            Self::Lambda => "lambda",
            Self::Branch => "branch",
            Self::Condition => "condition",
            Self::ConditionPart => "condition part",
            Self::MatchScrutinee => "match",
            Self::Result => "result",
            Self::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
pub struct SimpleBinding {
    pub name: String,
    #[serde(rename = "type")]
    pub type_str: String,
    pub binding_kind: String,
    #[serde(default)]
    pub navigation_locations: Option<PreresolvedNavigationTargets>,
}

impl SimpleBinding {
    pub fn kind(&self) -> BindingKind {
        match self.binding_kind.as_str() {
            "let_bind" => BindingKind::LetBind,
            "match_var" => BindingKind::MatchVar,
            "for_var" => BindingKind::ForVar,
            _ => BindingKind::FunParam,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataFlowNode {
    pub inner: GraphNode,
    pub diagnostics: Vec<DiagnosticInfo>,
}

impl DataFlowNode {
    pub fn new(inner: GraphNode) -> Self {
        let diagnostics = inner.diagnostics.clone();
        Self { inner, diagnostics }
    }

    pub fn data_flow_kind(&self) -> DataFlowKind {
        self.inner.kind.as_deref().map_or_else(
            || DataFlowKind::Custom(String::new()),
            DataFlowKind::from_str,
        )
    }

    /// Get all bindings in scope after this node.
    pub fn bindings_after(&self) -> Vec<SimpleBinding> {
        self.inner
            .metadata
            .get("bindings_after")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// Get indices of newly introduced bindings.
    pub fn new_binding_indices(&self) -> Vec<u64> {
        self.inner
            .metadata
            .get("new_binding_indices")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// Check if a binding at the given index is new (introduced by this node).
    pub fn is_new_binding(&self, idx: usize) -> bool {
        self.new_binding_indices().contains(&(idx as u64))
    }

    /// Get the expected type as a plain string (from metadata).
    pub fn expected_type(&self) -> Option<String> {
        self.inner
            .metadata
            .get("expected_type")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
    }

    /// Check if this is a leaf node (no children).
    pub fn is_leaf(&self) -> bool {
        self.inner.edges.is_empty()
    }

    /// Check if this node is complete (from metadata, not inferred).
    pub fn is_complete(&self) -> bool {
        self.inner
            .metadata
            .get("is_complete")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(self.inner.edges.is_empty())
    }

    /// Check if this node has any error diagnostics.
    pub fn has_error(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
    }
}

impl Deref for DataFlowNode {
    type Target = GraphNode;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DagNode for DataFlowNode {
    fn id(&self) -> NodeId {
        self.inner.id
    }

    fn children(&self) -> Vec<NodeId> {
        self.inner.edges.iter().map(|e| e.target).collect()
    }
}

impl NavNode for DataFlowNode {
    fn new_hyp_indices(&self) -> Vec<u64> {
        // Return all binding indices for navigation (not just newly introduced ones)
        // This allows keyboard navigation to select any binding in the node
        (0..self.bindings_after().len() as u64).collect()
    }

    fn goal_count(&self) -> usize {
        usize::from(self.expected_type().is_some())
    }

    fn has_selectable_goals(&self) -> bool {
        self.expected_type().is_some()
    }
}

impl NodeTextContent for DataFlowNode {
    fn title(&self) -> String {
        self.inner.content.to_plain_text()
    }

    fn content_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = self
            .bindings_after()
            .iter()
            .map(|b| format!("{}: {}", b.name, b.type_str))
            .collect();

        lines.push(
            self.expected_type()
                .map_or_else(|| "✓".to_string(), |t| format!("⊢ {t}")),
        );
        for d in &self.diagnostics {
            if d.severity == DiagnosticSeverity::Error {
                let msg = d.message.lines().next().unwrap_or(&d.message);
                lines.push(format!("✗ {msg}"));
            }
        }

        lines
    }
}

impl LayoutSize for DataFlowNode {
    fn layout_size(&self) -> (u16, u16) {
        calculate_node_size(self)
    }
}

#[derive(Debug)]
pub enum DataFlowConversionError {
    WrongDisplayStyle {
        expected: DisplayStyle,
        actual: DisplayStyle,
    },
}

impl fmt::Display for DataFlowConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongDisplayStyle { expected, actual } => {
                write!(f, "Expected display_style {expected:?}, got {actual:?}")
            }
        }
    }
}

impl Error for DataFlowConversionError {}

#[derive(Debug, Clone)]
pub struct DataFlowDag {
    pub nodes: Vec<DataFlowNode>,
    pub root_node_id: Option<NodeId>,
    pub current_node_id: Option<NodeId>,
    pub orphans: Vec<NodeId>,
    pub block_output: Option<RichText>,
    pub block_input: Vec<LocalBinding>,
}

impl DataFlowDag {
    pub fn get(&self, id: NodeId) -> Option<&DataFlowNode> {
        self.nodes.iter().find(|n| n.inner.id == id)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Iterate nodes in depth-first order.
    pub fn dfs_iter(&self) -> impl Iterator<Item = &DataFlowNode> {
        DataFlowDfsIterator {
            dag: self,
            stack: self.root_node_id.into_iter().collect(),
        }
    }

    /// Convert to petgraph-based `DagGraph` for rendering.
    pub fn to_graph(&self) -> DagGraph<DataFlowNode> {
        let mut graph = DagGraph::new();

        for node in &self.nodes {
            graph.add_node(node.id(), node.clone());
        }

        for node in &self.nodes {
            add_data_flow_edges(node, &mut graph);
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

    pub fn to_layout_graph(&self) -> LayoutGraph {
        let mut layout = LayoutGraph::new();
        let mut id_to_index = HashMap::new();

        for node in &self.nodes {
            let (width, height) = node.layout_size();
            let index = NodeIndex::new(layout.nodes.len());
            id_to_index.insert(node.id(), index);
            layout.add_node(index, node.id(), width, height);
        }

        for node in &self.nodes {
            add_data_flow_layout_edges(node, &id_to_index, &mut layout);
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
                .and_then(|node| node.bindings_after().get(hyp_idx).cloned())
                .and_then(|b| b.navigation_locations)
                .and_then(|nav| nav.definition)
                .map(|loc| (loc.uri, loc.position)),
            Selection::Goal { node_id, .. } => self
                .get(node_id)
                .map(|node| (current_uri.to_string(), node.position)),
            Selection::Theorem => self
                .root_node_id
                .and_then(|id| self.get(id))
                .map(|node| (current_uri.to_string(), node.position)),
            Selection::Error { .. } => None,
        }
    }
}

struct DataFlowDfsIterator<'a> {
    dag: &'a DataFlowDag,
    stack: Vec<NodeId>,
}

impl<'a> Iterator for DataFlowDfsIterator<'a> {
    type Item = &'a DataFlowNode;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.dag.get(id)?;
        for &child_id in node.children().iter().rev() {
            self.stack.push(child_id);
        }
        Some(node)
    }
}

fn add_data_flow_edges(node: &DataFlowNode, graph: &mut DagGraph<DataFlowNode>) {
    let Some(from_idx) = graph.index_of(node.id()) else {
        return;
    };
    for edge in &node.inner.edges {
        let Some(to_idx) = graph.index_of(edge.target) else {
            continue;
        };
        graph.add_edge(
            from_idx,
            to_idx,
            EdgeData {
                style: Some(Style::new().fg(Theme::PANE_BORDER_FOCUSED)),
                label: edge.label.clone(),
            },
        );
    }
}

fn add_data_flow_layout_edges(
    node: &DataFlowNode,
    id_to_index: &HashMap<NodeId, NodeIndex>,
    layout: &mut LayoutGraph,
) {
    let Some(&from_idx) = id_to_index.get(&node.id()) else {
        return;
    };
    for edge in &node.inner.edges {
        let Some(&to_idx) = id_to_index.get(&edge.target) else {
            continue;
        };
        layout.add_edge(from_idx, to_idx, edge.strength.map_or(1, NonZero::get));
    }
}

impl TryFrom<GenericDag> for DataFlowDag {
    type Error = DataFlowConversionError;

    fn try_from(dag: GenericDag) -> Result<Self, Self::Error> {
        if dag.display_style != DisplayStyle::Dataflow {
            return Err(DataFlowConversionError::WrongDisplayStyle {
                expected: DisplayStyle::Dataflow,
                actual: dag.display_style,
            });
        }

        let nodes: Vec<DataFlowNode> = dag.nodes.into_iter().map(DataFlowNode::new).collect();

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

impl Dag for DataFlowDag {
    type Node = DataFlowNode;

    fn get_node(&self, id: NodeId) -> Option<&Self::Node> {
        self.get(id)
    }
}
