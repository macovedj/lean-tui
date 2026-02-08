use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt,
    num::NonZero,
};

use petgraph::graph::NodeIndex;
use ratatui::style::{Color, Style};

use crate::{
    graph_layout::{
        Dag, DagGraph, DagNode, EdgeData, LayoutGraph, LayoutSize, NavNode, NodeTextContent,
    },
    protocol::{
        DiagnosticInfo, DiagnosticSeverity, DisplayStyle, GenericDag, GraphEdge, GraphNode,
        LocalBinding, NodeId, RichText,
    },
    theme::Theme,
};

/// Parsed bindings from node metadata.
fn parse_bindings(metadata: &serde_json::Map<String, serde_json::Value>) -> Vec<LocalBinding> {
    metadata
        .get("bindings")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// Effect node kinds from the lean-prism backend.
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

    pub fn icon(&self) -> &'static str {
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
}

#[derive(Debug, Clone)]
pub struct EffectFlowNode {
    pub id: NodeId,
    pub content: RichText,
    pub edges: Vec<GraphEdge>,
    pub effect_kind: EffectKind,
    pub forward: Option<String>,
    pub backward: Option<String>,
    pub bindings: Vec<LocalBinding>,
    pub diagnostics: Vec<DiagnosticInfo>,
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

        let bindings = parse_bindings(&node.metadata);
        let diagnostics = node.diagnostics.clone();

        Some(Self {
            id: node.id,
            content: node.content,
            edges: node.edges,
            effect_kind,
            forward: extract_str("forward"),
            backward: extract_str("backward"),
            bindings,
            diagnostics,
        })
    }

    pub fn is_leaf(&self) -> bool {
        self.edges.is_empty()
    }

    /// Check if this node has any error diagnostics.
    pub fn has_error(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == DiagnosticSeverity::Error)
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

impl NodeTextContent for EffectFlowNode {
    fn title(&self) -> String {
        self.content.to_plain_text()
    }

    fn content_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();

        // Add binding lines (filtered like in state_node)
        for b in &self.bindings {
            if !b.is_implicit && !b.is_instance {
                lines.push(format!("{}: {}", b.name, b.type_.to_plain_text()));
            }
        }

        // Add expression line - always present
        lines.push(self.content.to_plain_text());

        // Add forward/backward effect lines
        if let Some(fwd) = &self.forward {
            lines.push(format!("→ {fwd}"));
        }
        if let Some(bwd) = &self.backward {
            lines.push(format!("← {bwd}"));
        }

        lines.extend(format_error_diagnostics(&self.diagnostics));

        if self.is_leaf() && !self.has_error() && self.forward.is_none() && self.backward.is_none()
        {
            lines.push("✓".to_string());
        }

        lines
    }
}

impl LayoutSize for EffectFlowNode {
    fn layout_size(&self) -> (u16, u16) {
        const BORDER_AND_PADDING: usize = 4;
        const BORDER_HEIGHT: u16 = 2;

        let visible_bindings = self.visible_bindings();
        let shows_completed = self.shows_completed_marker();

        let content_width = [
            Self::binding_table_width(&visible_bindings),
            self.content.to_plain_text().chars().count(),
            self.effect_width(),
            if shows_completed {
                "✓ completed".chars().count()
            } else {
                0
            },
        ]
        .into_iter()
        .max()
        .unwrap_or(0);

        let line_count = visible_bindings.len()
            + 1 // expression
            + usize::from(self.forward.is_some())
            + usize::from(self.backward.is_some())
            + usize::from(shows_completed);

        let width = (content_width + BORDER_AND_PADDING) as u16;
        let height = BORDER_HEIGHT + line_count as u16;

        (width, height)
    }
}

impl EffectFlowNode {
    fn visible_bindings(&self) -> Vec<&LocalBinding> {
        self.bindings
            .iter()
            .filter(|b| !b.is_implicit && !b.is_instance)
            .collect()
    }

    fn binding_table_width(bindings: &[&LocalBinding]) -> usize {
        if bindings.is_empty() {
            return 0;
        }
        let max_name = bindings.iter().map(|b| b.name.chars().count()).max();
        let max_type = bindings
            .iter()
            .map(|b| b.type_.to_plain_text().chars().count())
            .max();

        max_name.unwrap_or(0) + 1 + max_type.unwrap_or(0)
    }

    fn effect_width(&self) -> usize {
        [&self.forward, &self.backward]
            .into_iter()
            .flatten()
            .map(|s| 2 + s.chars().count())
            .max()
            .unwrap_or(0)
    }

    fn shows_completed_marker(&self) -> bool {
        self.is_leaf() && !self.has_error() && self.forward.is_none() && self.backward.is_none()
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

fn format_error_diagnostics(diagnostics: &[DiagnosticInfo]) -> Vec<String> {
    diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .flat_map(|d| {
            d.message.lines().enumerate().map(|(i, line)| {
                let prefix = if i == 0 { "✗ " } else { "  " };
                format!("{prefix}{line}")
            })
        })
        .collect()
}

#[derive(Debug)]
pub enum EffectFlowConversionError {
    WrongDisplayStyle {
        expected: DisplayStyle,
        actual: DisplayStyle,
    },
    MissingEffectKind {
        node_id: u64,
    },
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
    pub block_input: Vec<LocalBinding>,
    pub block_output: Option<RichText>,
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

    /// Convert to petgraph-based `DagGraph` for rendering, optionally filtering
    /// out expand edges.
    pub fn to_graph_filtered(&self, include_expand: bool) -> DagGraph<EffectFlowNode> {
        let excluded_node_ids = self.expand_target_ids();
        let mut graph = DagGraph::new();

        for node in &self.nodes {
            if !include_expand && excluded_node_ids.contains(&node.id) {
                continue;
            }
            graph.add_node(node.id, node.clone());
        }

        for node in &self.nodes {
            if !include_expand && excluded_node_ids.contains(&node.id) {
                continue;
            }
            add_effect_edges_filtered(node, &mut graph, include_expand);
        }

        graph.root = self.root_node_id.and_then(|id| graph.index_of(id));
        graph.current = self.current_node_id.and_then(|id| graph.index_of(id));
        graph.orphans = self
            .orphans
            .iter()
            .filter(|id| include_expand || !excluded_node_ids.contains(id))
            .filter_map(|&id| graph.index_of(id))
            .collect();

        graph
    }

    /// Convert to layout graph, optionally filtering out expand edges.
    pub fn to_layout_graph_filtered(&self, include_expand: bool) -> LayoutGraph {
        let excluded_node_ids = self.expand_target_ids();
        let mut layout = LayoutGraph::new();
        let mut id_to_index = HashMap::new();

        for node in &self.nodes {
            if !include_expand && excluded_node_ids.contains(&node.id) {
                continue;
            }
            let (width, height) = node.layout_size();
            let index = NodeIndex::new(layout.nodes.len());
            id_to_index.insert(node.id, index);
            layout.add_node(index, node.id, width, height);
        }

        for node in &self.nodes {
            if !include_expand && excluded_node_ids.contains(&node.id) {
                continue;
            }
            add_effect_layout_edges_filtered(node, &id_to_index, &mut layout, include_expand);
        }

        layout.root = self
            .root_node_id
            .and_then(|id| id_to_index.get(&id).copied());
        layout.orphans = self
            .orphans
            .iter()
            .filter(|id| include_expand || !excluded_node_ids.contains(id))
            .filter_map(|id| id_to_index.get(id).copied())
            .collect();

        layout
    }

    /// Get all node IDs that should be hidden when expand is collapsed.
    /// This includes direct targets of "expand" edges and all their
    /// descendants.
    fn expand_target_ids(&self) -> HashSet<NodeId> {
        let roots = self.direct_expand_targets();
        self.reachable_from(&roots)
    }

    /// Get node IDs that are direct targets of "expand" edges.
    fn direct_expand_targets(&self) -> HashSet<NodeId> {
        self.nodes
            .iter()
            .flat_map(|node| &node.edges)
            .filter(|e| e.label.as_deref() == Some("expand"))
            .map(|e| e.target)
            .collect()
    }

    /// Get all nodes reachable from a set of root nodes (inclusive).
    fn reachable_from(&self, roots: &HashSet<NodeId>) -> HashSet<NodeId> {
        let mut visited = roots.clone();
        let mut frontier: Vec<NodeId> = roots.iter().copied().collect();

        while let Some(node_id) = frontier.pop() {
            let unvisited_children = self
                .get(node_id)
                .into_iter()
                .flat_map(|node| node.edges.iter().map(|e| e.target))
                .filter(|&id| visited.insert(id));
            frontier.extend(unvisited_children);
        }

        visited
    }

    /// Get the number of nodes, optionally excluding expand targets.
    pub fn node_count_filtered(&self, include_expand: bool) -> usize {
        if include_expand {
            self.nodes.len()
        } else {
            let excluded = self.expand_target_ids();
            self.nodes
                .iter()
                .filter(|n| !excluded.contains(&n.id))
                .count()
        }
    }
}

fn add_effect_edges_filtered(
    node: &EffectFlowNode,
    graph: &mut DagGraph<EffectFlowNode>,
    include_expand: bool,
) {
    let Some(from_idx) = graph.index_of(node.id) else {
        return;
    };
    for edge in &node.edges {
        if !include_expand && edge.label.as_deref() == Some("expand") {
            continue;
        }
        let Some(to_idx) = graph.index_of(edge.target) else {
            continue;
        };
        let edge_kind = edge.effect_edge_kind();
        graph.add_edge(
            from_idx,
            to_idx,
            EdgeData {
                style: Some(Style::new().fg(edge_kind.color())),
                label: edge.label.clone(),
            },
        );
    }
}

fn add_effect_layout_edges_filtered(
    node: &EffectFlowNode,
    id_to_index: &HashMap<NodeId, NodeIndex>,
    layout: &mut LayoutGraph,
    include_expand: bool,
) {
    let Some(&from_idx) = id_to_index.get(&node.id) else {
        return;
    };
    for edge in &node.edges {
        if !include_expand && edge.label.as_deref() == Some("expand") {
            continue;
        }
        let Some(&to_idx) = id_to_index.get(&edge.target) else {
            continue;
        };
        layout.add_edge(from_idx, to_idx, edge.strength.map_or(1, NonZero::get));
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
            block_input: dag.block_input,
            block_output: dag.block_output,
        })
    }
}

impl Dag for EffectFlowDag {
    type Node = EffectFlowNode;

    fn get_node(&self, id: NodeId) -> Option<&Self::Node> {
        self.get(id)
    }
}
