use std::{error::Error, fmt, ops::Deref};

use ratatui::style::Color;

use crate::{
    protocol::{
        AnnotatedTextTree, Dag, DagNode, DisplayStyle, GenericDag, GraphNode, LocalBinding,
        NavNode, NodeId,
    },
    shared::{LayoutSize, Theme},
};

/// Data flow node kinds from the lean-dag backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataFlowKind {
    Binding,
    Lambda,
    Branch,
    Condition,
    Custom(String),
}

impl DataFlowKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "binding" => Self::Binding,
            "lambda" => Self::Lambda,
            "branch" => Self::Branch,
            "condition" => Self::Condition,
            other => Self::Custom(other.to_owned()),
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::Binding => "≔",
            Self::Lambda => "λ",
            Self::Branch => "⎇",
            Self::Condition => "?",
            Self::Custom(_) => "◆",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::Binding => Theme::DATAFLOW_BINDING,
            Self::Lambda => Theme::DATAFLOW_LAMBDA,
            Self::Branch => Theme::DATAFLOW_BRANCH,
            Self::Condition => Theme::DATAFLOW_CONDITION,
            Self::Custom(_) => Color::Gray,
        }
    }

    pub fn description(&self) -> &str {
        match self {
            Self::Binding => "binding",
            Self::Lambda => "lambda",
            Self::Branch => "branch",
            Self::Condition => "condition",
            Self::Custom(s) => s,
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

    pub fn bindings_after(&self) -> Vec<LocalBinding> {
        self.inner
            .metadata
            .get("bindings_after")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    pub fn new_binding_indices(&self) -> Vec<u64> {
        self.inner
            .metadata
            .get("new_binding_indices")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    pub fn expected_type(&self) -> Option<AnnotatedTextTree> {
        self.inner
            .metadata
            .get("expected_type")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    pub fn is_leaf(&self) -> bool {
        self.inner.edges.is_empty()
    }

    pub fn is_complete(&self) -> bool {
        self.inner.edges.is_empty() && self.expected_type().is_none()
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
        self.new_binding_indices()
    }

    fn goal_count(&self) -> usize {
        usize::from(self.expected_type().is_some())
    }

    fn has_selectable_goals(&self) -> bool {
        self.expected_type().is_some()
    }
}

impl LayoutSize for DataFlowNode {
    fn layout_size(&self) -> (u16, u16) {
        let width = {
            let title_width = self.inner.content.to_plain_text().len() + 8;
            (title_width + 4) as u16
        };
        let height = {
            // Border (2) + bindings line (if any) + type line (1)
            let has_bindings = !self.new_binding_indices().is_empty();
            let content_lines = if has_bindings { 2 } else { 1 };
            2 + content_lines
        };
        (width, height)
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

    /// Check if a node is the current node.
    pub fn is_current(&self, node_id: NodeId) -> bool {
        self.current_node_id == Some(node_id)
    }

    /// Iterate nodes in depth-first order.
    pub fn dfs_iter(&self) -> impl Iterator<Item = &DataFlowNode> {
        DataFlowDfsIterator {
            dag: self,
            stack: self.root_node_id.into_iter().collect(),
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

        let initial_context: Vec<LocalBinding> = dag
            .metadata
            .get("initial_context")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        Ok(Self {
            nodes,
            root_node_id: dag.root_node_id,
            current_node_id: dag.current_node_id,
            orphans: dag.orphans,
            definition_type: dag.definition_type,
            initial_context,
        })
    }
}

impl Dag for DataFlowDag {
    type Node = DataFlowNode;

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
