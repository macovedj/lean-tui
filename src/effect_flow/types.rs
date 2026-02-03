use std::{collections::HashSet, error::Error, fmt};

use ratatui::style::Color;
use tracing::debug;

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

    pub fn description(&self) -> &str {
        match self {
            Self::EffectBind => "bind",
            Self::StateRead => "read",
            Self::StateWrite => "write",
            Self::StateModify => "modify",
            Self::EffectReturn => "return",
            Self::EffectThrow => "throw",
            Self::EffectCatch => "catch",
            Self::Pure => "pure",
            Self::Custom(s) => s,
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

    /// Check if this is a continuation edge.
    pub const fn is_continuation(self) -> bool {
        matches!(self, Self::Continuation)
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
    pub state_effect: Option<String>,
    pub forward_output: Option<String>,
    pub given_back: Option<String>,
    pub forward_desc: Option<String>,
    pub backward_desc: Option<String>,
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

    /// Check if this is a continuation edge.
    pub fn is_continuation(&self) -> bool {
        self.effect_edge_kind().is_continuation()
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

    /// Check if a node is the current node.
    pub fn is_current(&self, node_id: NodeId) -> bool {
        self.current_node_id == Some(node_id)
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

    /// Extract the main flow (continuation chain) from root.
    pub fn extract_main_flow(&self) -> MainFlow {
        MainFlow::extract_from(self)
    }

    /// Get detail children for a node (non-continuation edges).
    pub fn detail_children(&self, node_id: NodeId) -> Vec<NodeId> {
        self.get(node_id)
            .map(|node| {
                node.edges
                    .iter()
                    .filter(|e| !e.is_continuation())
                    .map(|e| e.target)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Create a sub-DAG containing only the detail nodes (non-continuation
    /// subtree) rooted at the given node.
    pub fn detail_subdag(&self, node_id: NodeId) -> DetailSubdag {
        DetailSubdag::extract_from(self, node_id)
    }

    /// Collect node and all its descendants via non-continuation edges only.
    /// This is used for building detail subdags that exclude the main flow.
    fn collect_detail_descendants(&self, node_id: NodeId, collected: &mut HashSet<NodeId>) {
        if !collected.insert(node_id) {
            return;
        }

        if let Some(node) = self.get(node_id) {
            for edge in &node.edges {
                // Only follow non-continuation edges for detail view
                if !edge.is_continuation() {
                    self.collect_detail_descendants(edge.target, collected);
                }
            }
        }
    }
}

/// The main sequential flow of operations extracted from an effect DAG.
/// This follows continuation edges to form the "spine" of the computation.
#[derive(Debug, Clone)]
pub struct MainFlow {
    node_ids: Vec<NodeId>,
}

impl MainFlow {
    /// Extract the main flow from a DAG by following continuation edges.
    pub fn extract_from(dag: &EffectFlowDag) -> Self {
        let mut node_ids = Vec::new();
        let Some(root_id) = dag.root_node_id else {
            return Self { node_ids };
        };

        let mut current_id = Some(root_id);
        while let Some(id) = current_id {
            node_ids.push(id);
            current_id = dag.get(id).and_then(|node| {
                node.edges
                    .iter()
                    .find(|e| e.is_continuation())
                    .map(|e| e.target)
            });
        }

        Self { node_ids }
    }

    /// Get the node IDs in order.
    pub fn node_ids(&self) -> &[NodeId] {
        &self.node_ids
    }

    /// Check if a node is part of the main flow.
    pub fn contains(&self, node_id: NodeId) -> bool {
        self.node_ids.contains(&node_id)
    }

    /// Get the index of a node in the main flow.
    pub fn index_of(&self, node_id: NodeId) -> Option<usize> {
        self.node_ids.iter().position(|&id| id == node_id)
    }

    /// Get the number of steps in the main flow.
    pub fn len(&self) -> usize {
        self.node_ids.len()
    }

    /// Check if the main flow is empty.
    pub fn is_empty(&self) -> bool {
        self.node_ids.is_empty()
    }

    /// Iterate over node IDs with their indices.
    pub fn iter_enumerated(&self) -> impl Iterator<Item = (usize, NodeId)> + '_ {
        self.node_ids.iter().copied().enumerate()
    }
}

/// A sub-DAG containing the detail nodes (non-continuation children) for a main
/// flow node.
#[derive(Debug, Clone)]
pub struct DetailSubdag {
    pub dag: EffectFlowDag,
    pub parent_node_id: NodeId,
}

impl DetailSubdag {
    /// Extract detail subtree from the given parent node.
    pub fn extract_from(dag: &EffectFlowDag, parent_node_id: NodeId) -> Self {
        let detail_children = dag.detail_children(parent_node_id);

        debug!(
            "DetailSubdag::extract_from: parent={}, detail_children={:?}",
            parent_node_id, detail_children
        );

        if detail_children.is_empty() {
            debug!("  no detail children, returning empty");
            return Self {
                dag: EffectFlowDag {
                    nodes: Vec::new(),
                    root_node_id: None,
                    current_node_id: dag.current_node_id,
                    orphans: Vec::new(),
                    metadata: dag.metadata.clone(),
                    initial_context: Vec::new(),
                    definition_type: None,
                },
                parent_node_id,
            };
        }

        // Collect all nodes reachable from detail edges (non-continuation only)
        let mut included_ids = HashSet::new();
        for child_id in &detail_children {
            dag.collect_detail_descendants(*child_id, &mut included_ids);
        }
        debug!("  included_ids after collect_detail_descendants: {:?}", included_ids);

        // Clone nodes but filter edges to only include targets within the subdag
        // This removes edges that point to the main flow or other parts of the graph
        let filtered_nodes: Vec<EffectFlowNode> = dag
            .nodes
            .iter()
            .filter(|n| included_ids.contains(&n.id))
            .map(|n| {
                let mut node = n.clone();
                node.edges.retain(|e| included_ids.contains(&e.target));
                node
            })
            .collect();

        Self {
            dag: EffectFlowDag {
                nodes: filtered_nodes,
                root_node_id: detail_children.first().copied(),
                current_node_id: dag.current_node_id.filter(|id| included_ids.contains(id)),
                orphans: detail_children.into_iter().skip(1).collect(),
                metadata: dag.metadata.clone(),
                initial_context: Vec::new(),
                definition_type: None,
            },
            parent_node_id,
        }
    }

    /// Check if this detail view has any nodes.
    pub fn is_empty(&self) -> bool {
        self.dag.is_empty()
    }

    /// Get the root nodes (direct detail children of the parent).
    pub fn root_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        self.dag
            .root_node_id
            .into_iter()
            .chain(self.dag.orphans.iter().copied())
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
