use std::{error::Error, fmt, ops::Deref};

use ratatui::style::Color;

use crate::{
    graph::{calculate_node_size, Dag, DagNode, NavNode, NodeTextContent},
    protocol::{
        AnnotatedTextTree, BindingKind, DisplayStyle, GenericDag, GraphNode, LocalBinding, NodeId,
        PreresolvedNavigationTargets,
    },
    shared::{LayoutSize, Theme},
};

/// Data flow node kinds from the lean-dag backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataFlowKind {
    /// Let binding: `let x := v; body`
    Binding,
    /// Lambda abstraction: `fun x => body`
    Lambda,
    /// Branch in control flow (then/else/match arm)
    Branch,
    /// Condition being tested (if condition)
    Condition,
    /// Part of a compound condition (e.g., left side of &&)
    ConditionPart,
    /// Match scrutinee expression
    MatchScrutinee,
    /// Final expression (leaf node)
    Result,
    /// Unknown kind
    Custom(String),
}

impl DataFlowKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "binding" => Self::Binding,
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
            Self::Binding => "≔",
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
            Self::Binding => Theme::DATAFLOW_BINDING,
            Self::Lambda => Theme::DATAFLOW_LAMBDA,
            Self::Branch => Theme::DATAFLOW_BRANCH,
            Self::Condition => Theme::DATAFLOW_CONDITION,
            Self::ConditionPart => Theme::DATAFLOW_CONDITION,
            Self::MatchScrutinee => Theme::DATAFLOW_BRANCH,
            Self::Result => Color::Green,
            Self::Custom(_) => Color::Gray,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Binding => "binding",
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

/// Simplified binding from dataflow metadata (type is plain string).
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SimpleBinding {
    pub name: String,
    #[allow(dead_code)]
    pub id: String,
    #[serde(rename = "type")]
    pub type_str: String,
    pub binding_kind: String,
    #[serde(default)]
    pub navigation_locations: Option<PreresolvedNavigationTargets>,
}

impl SimpleBinding {
    /// Convert binding_kind string to BindingKind enum.
    pub fn kind(&self) -> BindingKind {
        match self.binding_kind.as_str() {
            "let_bind" => BindingKind::LetBind,
            "fun_param" => BindingKind::FunParam,
            "match_var" => BindingKind::MatchVar,
            "for_var" => BindingKind::ForVar,
            _ => BindingKind::FunParam,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataFlowNode {
    pub inner: GraphNode,
}

impl DataFlowNode {
    pub const fn new(inner: GraphNode) -> Self {
        Self { inner }
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
            .and_then(|v| v.as_bool())
            .unwrap_or(self.inner.edges.is_empty())
    }

    /// Get the branch label (for branch nodes like then/else/match arm).
    pub fn branch_label(&self) -> Option<String> {
        self.inner
            .metadata
            .get("branch_label")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(String::from)
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
                .map(|t| format!("⊢ {t}"))
                .unwrap_or_else(|| "✓".to_string()),
        );

        lines
    }
}

impl LayoutSize for DataFlowNode {
    fn layout_size(&self) -> (u16, u16) {
        calculate_node_size(self)
    }
}

/// Error when converting a `GenericDag` to `DataFlowDag`.
#[derive(Debug)]
pub enum DataFlowConversionError {
    /// Wrong display style.
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
    pub definition_type: Option<AnnotatedTextTree>,
    pub initial_context: Vec<LocalBinding>,
}

impl DataFlowDag {
    /// Get a node by ID.
    pub fn get(&self, id: NodeId) -> Option<&DataFlowNode> {
        self.nodes.iter().find(|n| n.inner.id == id)
    }

    /// Get the number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if empty.
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

    /// Convert to petgraph-based DagGraph for rendering.
    pub fn to_graph(&self) -> crate::graph::DagGraph<DataFlowNode> {
        use ratatui::style::Style;

        use crate::graph::{DagGraph, EdgeData};

        let mut graph = DagGraph::new();

        for node in &self.nodes {
            graph.add_node(node.id(), node.clone());
        }

        for node in &self.nodes {
            let Some(from_idx) = graph.index_of(node.id()) else {
                continue;
            };
            for edge in &node.inner.edges {
                let Some(to_idx) = graph.index_of(edge.target) else {
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

    /// Convert to layout graph for force-directed layout computation.
    pub fn to_layout_graph(&self) -> crate::graph::LayoutGraph {
        use crate::{graph::LayoutGraph, shared::LayoutSize};

        let mut layout = LayoutGraph::new();
        let mut id_to_index = std::collections::HashMap::new();

        for node in &self.nodes {
            let (width, height) = node.layout_size();
            let index = petgraph::graph::NodeIndex::new(layout.nodes.len());
            id_to_index.insert(node.id(), index);
            layout.add_node(index, node.id(), width, height);
        }

        for node in &self.nodes {
            let Some(&from_idx) = id_to_index.get(&node.id()) else {
                continue;
            };
            for edge in &node.inner.edges {
                let Some(&to_idx) = id_to_index.get(&edge.target) else {
                    continue;
                };
                layout.add_edge(
                    from_idx,
                    to_idx,
                    edge.strength.map_or(1, std::num::NonZero::get),
                );
            }
        }

        if let Some(root_id) = self.root_node_id {
            layout.root = id_to_index.get(&root_id).copied();
        }
        for &orphan_id in &self.orphans {
            if let Some(&idx) = id_to_index.get(&orphan_id) {
                layout.orphans.push(idx);
            }
        }

        layout
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
            definition_type: dag.definition_type,
            initial_context: dag.initial_context,
        })
    }
}

impl Dag for DataFlowDag {
    type Node = DataFlowNode;

    fn get_node(&self, id: NodeId) -> Option<&Self::Node> {
        self.get(id)
    }
}
