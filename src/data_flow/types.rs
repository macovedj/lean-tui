use ratatui::style::Color;

use crate::{
    protocol::{
        AnnotatedTextTree, Dag, DagNode, DisplayStyle, GenericDag, GraphNode, LocalBinding, NodeId,
    },
    shared::Theme,
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

/// Extension trait for `GraphNode` in dataflow mode.
pub trait DataFlowNodeExt {
    fn data_flow_kind(&self) -> DataFlowKind;
    fn bindings_after(&self) -> Vec<LocalBinding>;
    fn new_binding_indices(&self) -> Vec<u64>;
    fn expected_type(&self) -> Option<AnnotatedTextTree>;
    fn is_leaf(&self) -> bool;
    fn is_complete(&self) -> bool;
}

impl DataFlowNodeExt for GraphNode {
    fn data_flow_kind(&self) -> DataFlowKind {
        self.kind.as_deref().map_or_else(
            || DataFlowKind::Custom(String::new()),
            DataFlowKind::from_str,
        )
    }

    fn bindings_after(&self) -> Vec<LocalBinding> {
        self.metadata
            .get("bindings_after")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    fn new_binding_indices(&self) -> Vec<u64> {
        self.metadata
            .get("new_binding_indices")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    fn expected_type(&self) -> Option<AnnotatedTextTree> {
        self.metadata
            .get("expected_type")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    fn is_leaf(&self) -> bool {
        self.edges.is_empty()
    }

    fn is_complete(&self) -> bool {
        self.edges.is_empty() && self.expected_type().is_none()
    }
}

// --- Validated DataFlowDag ---

use std::{error::Error, fmt, ops::Deref};

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

/// Validated data flow DAG.
#[derive(Debug, Clone)]
pub struct DataFlowDag {
    /// The underlying generic DAG.
    pub inner: GenericDag,
}

impl DataFlowDag {
    /// Get the number of nodes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Deref for DataFlowDag {
    type Target = GenericDag;

    fn deref(&self) -> &Self::Target {
        &self.inner
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

        Ok(Self { inner: dag })
    }
}

impl Dag for DataFlowDag {
    type Node = GraphNode;

    fn get_node(&self, id: NodeId) -> Option<&Self::Node> {
        self.inner.get(id)
    }

    fn root_id(&self) -> Option<NodeId> {
        self.inner.root_node_id
    }

    fn orphan_ids(&self) -> &[NodeId] {
        &self.inner.orphans
    }
}

impl DagNode for DataFlowDag {
    fn id(&self) -> NodeId {
        self.inner.root_node_id.unwrap_or(0)
    }

    fn children(&self) -> Vec<NodeId> {
        self.inner
            .root_node_id
            .and_then(|id| self.inner.get(id))
            .map(|n| n.edges.iter().map(|e| e.target).collect())
            .unwrap_or_default()
    }

    fn box_height(&self) -> u16 {
        3
    }

    fn content_width(&self) -> u16 {
        40
    }
}
