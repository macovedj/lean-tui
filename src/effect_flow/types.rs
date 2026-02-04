use std::{error::Error, fmt};

use ratatui::style::Color;

use crate::{
    protocol::{
        self, Dag, DagNode, DisplayStyle, GenericDag, GraphEdge, GraphNode, NavNode, NodeId,
    },
    shared::{LayoutSize, Theme},
};

/// Effect node kinds from the lean-dag backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EffectKind {
    EffectBind,
    EffectBranch,
    StateRead,
    StateWrite,
    StateModify,
    EffectReturn,
    EffectThrow,
    EffectCatch,
    Pure,
    Custom(String),
}

impl EffectKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "effectBind" => Self::EffectBind,
            "effectBranch" => Self::EffectBranch,
            "stateRead" => Self::StateRead,
            "stateWrite" => Self::StateWrite,
            "stateModify" => Self::StateModify,
            "effectReturn" => Self::EffectReturn,
            "effectThrow" => Self::EffectThrow,
            "effectCatch" => Self::EffectCatch,
            "pure" => Self::Pure,
            other => Self::Custom(other.to_owned()),
        }
    }

    pub fn icon(&self) -> &str {
        match self {
            Self::EffectBind => "⊛",
            Self::EffectBranch => "⎇",
            Self::StateRead => "◁",
            Self::StateWrite => "▷",
            Self::StateModify => "◇",
            Self::EffectReturn => "↵",
            Self::EffectThrow => "⚡",
            Self::EffectCatch => "⛨",
            Self::Pure => "●",
            Self::Custom(_) => "◆",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Self::EffectBind => Theme::EFFECT_BIND,
            Self::EffectBranch => Theme::EDGE_CONTROL_FLOW,
            Self::StateRead => Theme::EFFECT_STATE_READ,
            Self::StateWrite => Theme::EFFECT_STATE_WRITE,
            Self::StateModify => Theme::EFFECT_STATE_MODIFY,
            Self::EffectReturn => Theme::EFFECT_RETURN,
            Self::EffectThrow => Theme::EFFECT_THROW,
            Self::EffectCatch => Theme::EFFECT_CATCH,
            Self::Pure => Theme::EFFECT_PURE,
            Self::Custom(_) => Color::Gray,
        }
    }
}

/// Edge kinds for effect flow edges.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeKind {
    /// Data flows from one computation to another
    DataFlow,
    /// Control flow (branching, sequencing)
    ControlFlow,
    /// Effect flow
    EffectFlow,
    /// Error propagation path
    ErrorPropagation,
    /// Early return path
    EarlyReturn,
    /// Continuation - "what happens next" in a bind chain
    Continuation,
    /// Unknown edge kind
    Unknown,
}

impl EdgeKind {
    /// Parse from an edge kind string.
    pub fn from_str(s: &str) -> Self {
        match s {
            "data_flow" => Self::DataFlow,
            "control_flow" => Self::ControlFlow,
            "effect_flow" => Self::EffectFlow,
            "error_propagation" => Self::ErrorPropagation,
            "early_return" => Self::EarlyReturn,
            "continuation" => Self::Continuation,
            _ => Self::Unknown,
        }
    }

    /// Get the color for this edge kind (from Theme).
    pub const fn color(self) -> Color {
        match self {
            Self::DataFlow => Theme::EDGE_DATA_FLOW,
            Self::ControlFlow => Theme::EDGE_CONTROL_FLOW,
            Self::EffectFlow => Theme::EDGE_EFFECT_FLOW,
            Self::ErrorPropagation => Theme::EDGE_ERROR_PROPAGATION,
            Self::EarlyReturn => Theme::EDGE_EARLY_RETURN,
            Self::Continuation => Theme::EDGE_CONTINUATION,
            Self::Unknown => Color::DarkGray,
        }
    }

    /// Get a label for this edge kind to display on the graph.
    pub const fn label(self) -> Option<&'static str> {
        match self {
            Self::Continuation => Some("then"),
            Self::DataFlow => Some("expand"),
            Self::ControlFlow => Some("branch"),
            Self::EffectFlow => Some("effect"),
            Self::ErrorPropagation => Some("error"),
            Self::EarlyReturn => Some("return"),
            Self::Unknown => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct EffectFlowNode {
    pub id: NodeId,
    pub content: protocol::AnnotatedTextTree,
    pub edges: Vec<GraphEdge>,
    pub effect_kind: EffectKind,
    pub forward: Option<String>,
    pub backward: Option<String>,
    #[allow(dead_code)]
    pub state_effect: Option<String>,
    #[allow(dead_code)]
    pub forward_output: Option<String>,
    #[allow(dead_code)]
    pub given_back: Option<String>,
    #[allow(dead_code)]
    pub forward_desc: Option<String>,
    #[allow(dead_code)]
    pub backward_desc: Option<String>,
    #[allow(dead_code)]
    pub from_unfolding: bool,
}

impl EffectFlowNode {
    pub fn from_graph_node(node: GraphNode) -> Option<Self> {
        let effect_kind = node.kind.as_deref().map(EffectKind::from_str)?;

        let extract_str = |key: &str| -> Option<String> {
            node.metadata
                .get(key)
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(String::from)
        };

        let state_effect = extract_str("stateEffect").filter(|s| s != "none");

        let from_unfolding = node
            .metadata
            .get("fromUnfolding")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        Some(Self {
            id: node.id,
            content: node.content,
            edges: node.edges,
            effect_kind,
            forward: extract_str("forward"),
            backward: extract_str("backward"),
            state_effect,
            forward_output: extract_str("forwardOutput"),
            given_back: extract_str("givenBack"),
            forward_desc: extract_str("forwardDesc"),
            backward_desc: extract_str("backwardDesc"),
            from_unfolding,
        })
    }

    pub fn is_leaf(&self) -> bool {
        self.edges.is_empty()
    }
}

impl DagNode for EffectFlowNode {
    fn id(&self) -> NodeId {
        self.id
    }

    fn children(&self) -> Vec<NodeId> {
        self.edges.iter().map(|e| e.target).collect()
    }
}

impl NavNode for EffectFlowNode {
    fn new_hyp_indices(&self) -> Vec<u64> {
        Vec::new()
    }

    fn goal_count(&self) -> usize {
        usize::from(!self.is_leaf())
    }

    fn has_selectable_goals(&self) -> bool {
        true
    }
}

impl LayoutSize for EffectFlowNode {
    fn layout_size(&self) -> (u16, u16) {
        let title_width = self.content.to_plain_text().len() + 12;
        let fwd_width = self.forward.as_ref().map_or(0, |s| s.len() + 6);
        let bwd_width = self.backward.as_ref().map_or(0, |s| s.len() + 6);
        let width = title_width.max(fwd_width).max(bwd_width).min(80) as u16;

        let content_lines = match (&self.forward, &self.backward) {
            (Some(_), Some(_)) => 2,
            (Some(_), None) | (None, Some(_)) => 1,
            (None, None) => 0,
        };
        (width, 2 + content_lines.max(1))
    }
}

impl GraphEdge {
    /// Get the edge kind for effect flow rendering.
    pub fn effect_edge_kind(&self) -> EdgeKind {
        self.kind
            .as_deref()
            .map_or(EdgeKind::Unknown, EdgeKind::from_str)
    }
}

#[derive(Debug)]
pub enum EffectFlowConversionError {
    /// Wrong display style.
    WrongDisplayStyle {
        expected: DisplayStyle,
        actual: DisplayStyle,
    },
    /// Node missing required effect kind metadata.
    MissingEffectKind { node_id: u64 },
}

impl fmt::Display for EffectFlowConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongDisplayStyle { expected, actual } => {
                write!(f, "Expected display_style {expected:?}, got {actual:?}")
            }
            Self::MissingEffectKind { node_id } => {
                write!(
                    f,
                    "Node {node_id}: missing required 'kind' field for effect flow mode"
                )
            }
        }
    }
}

impl Error for EffectFlowConversionError {}

#[derive(Debug, Clone)]
pub struct EffectFlowDag {
    pub nodes: Vec<EffectFlowNode>,
    pub root_node_id: Option<NodeId>,
    pub current_node_id: Option<NodeId>,
    pub orphans: Vec<NodeId>,
    pub metadata: serde_json::Map<String, serde_json::Value>,
    pub initial_context: Vec<protocol::LocalBinding>,
    pub definition_type: Option<protocol::AnnotatedTextTree>,
}

impl EffectFlowDag {
    /// Get a node by ID.
    pub fn get(&self, id: NodeId) -> Option<&EffectFlowNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get the number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the monad stack from DAG metadata.
    pub fn monad_stack(&self) -> Option<Vec<String>> {
        self.metadata
            .get("monad_stack")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Iterate nodes in depth-first order.
    pub fn dfs_iter(&self) -> impl Iterator<Item = &EffectFlowNode> {
        EffectFlowDfsIterator {
            dag: self,
            stack: self.root_node_id.into_iter().collect(),
        }
    }

    /// Convert to petgraph-based DagGraph for rendering.
    pub fn to_graph(&self) -> crate::graph::DagGraph<EffectFlowNode> {
        use crate::graph::{DagGraph, EdgeData};
        use ratatui::style::Style;

        let mut graph = DagGraph::new();

        // Add all nodes
        for node in &self.nodes {
            graph.add_node(node.id, node.clone());
        }

        // Add edges with style data
        for node in &self.nodes {
            let Some(from_idx) = graph.index_of(node.id) else {
                continue;
            };
            for edge in &node.edges {
                let Some(to_idx) = graph.index_of(edge.target) else {
                    continue;
                };
                let edge_kind = edge.effect_edge_kind();
                graph.add_edge(
                    from_idx,
                    to_idx,
                    EdgeData {
                        style: Some(Style::new().fg(edge_kind.color())),
                        label: edge_kind.label(),
                    },
                );
            }
        }

        // Set root and current
        if let Some(root_id) = self.root_node_id {
            if let Some(idx) = graph.index_of(root_id) {
                graph.root = Some(idx);
            }
        }
        if let Some(current_id) = self.current_node_id {
            graph.current = graph.index_of(current_id);
        }

        // Add orphans
        for &orphan_id in &self.orphans {
            if let Some(idx) = graph.index_of(orphan_id) {
                graph.orphans.push(idx);
            }
        }

        graph
    }
}

struct EffectFlowDfsIterator<'a> {
    dag: &'a EffectFlowDag,
    stack: Vec<NodeId>,
}

impl<'a> Iterator for EffectFlowDfsIterator<'a> {
    type Item = &'a EffectFlowNode;

    fn next(&mut self) -> Option<Self::Item> {
        let id = self.stack.pop()?;
        let node = self.dag.get(id)?;
        for &child_id in node.children().iter().rev() {
            self.stack.push(child_id);
        }
        Some(node)
    }
}

impl TryFrom<GenericDag> for EffectFlowDag {
    type Error = EffectFlowConversionError;

    fn try_from(dag: GenericDag) -> Result<Self, Self::Error> {
        if dag.display_style != DisplayStyle::Effectflow {
            return Err(EffectFlowConversionError::WrongDisplayStyle {
                expected: DisplayStyle::Effectflow,
                actual: dag.display_style,
            });
        }

        let nodes: Vec<EffectFlowNode> = dag
            .nodes
            .into_iter()
            .map(|node| {
                let node_id = node.id;
                EffectFlowNode::from_graph_node(node)
                    .ok_or(EffectFlowConversionError::MissingEffectKind { node_id })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            nodes,
            root_node_id: dag.root_node_id,
            current_node_id: dag.current_node_id,
            orphans: dag.orphans,
            metadata: dag.metadata,
            initial_context: dag.initial_context,
            definition_type: dag.definition_type,
        })
    }
}

impl Dag for EffectFlowDag {
    type Node = EffectFlowNode;

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
