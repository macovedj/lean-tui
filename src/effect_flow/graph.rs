use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fmt, iter,
    num::NonZero,
};

use petgraph::graph::NodeIndex;
use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::{
    graph_layout::{Dag, DagGraph, DagNode, EdgeData, LayoutGraph, NavNode},
    protocol::{
        range_contains_cursor, range_size, DiagnosticInfo, DiagnosticSeverity, DisplayStyle,
        GenericDag, GraphEdge, GraphNode, LineCharacterPosition, LocalBinding, NodeId, RichText,
        TimingDisplay,
    },
    selection::Selection,
    theme::{Red, Theme},
    widgets::{
        aligned_table::AlignedRow,
        diff_text::RichTextExt,
        node_content::{ContentItem, ErrorLine, NodeContent, NodeStyle, NodeVisualState},
    },
};

/// Parsed bindings from node metadata.
fn parse_bindings(metadata: &serde_json::Map<String, serde_json::Value>) -> Vec<LocalBinding> {
    metadata
        .get("bindings")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

/// An effect value with kind and value.
#[derive(Debug, Clone)]
pub struct EffectValue {
    pub kind: EffectKind,
    pub value: String,
}

fn parse_effect_values(
    metadata: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Vec<EffectValue> {
    metadata
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    let obj = v.as_object()?;
                    let kind = obj.get("kind")?.as_str().map(EffectKind::from_str)?;
                    let value = obj.get("value")?.as_str()?.to_string();
                    Some(EffectValue { kind, value })
                })
                .collect()
        })
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
    pub position: LineCharacterPosition,
    pub end_position: Option<LineCharacterPosition>,
    pub edges: Vec<GraphEdge>,
    pub effect_kind: EffectKind,
    pub forwards: Vec<EffectValue>,
    pub backwards: Vec<EffectValue>,
    pub bindings: Vec<LocalBinding>,
    pub diagnostics: Vec<DiagnosticInfo>,
    pub self_elaboration_ns: Option<u64>,
    pub self_elaboration_heartbeats: Option<u64>,
}

impl EffectFlowNode {
    pub fn from_graph_node(node: GraphNode) -> Option<Self> {
        let effect_kind = node.kind.as_deref().map(EffectKind::from_str)?;

        let bindings = parse_bindings(&node.metadata);
        let forwards = parse_effect_values(&node.metadata, "forwards");
        let backwards = parse_effect_values(&node.metadata, "backwards");
        let diagnostics = node.diagnostics.clone();

        Some(Self {
            id: node.id,
            self_elaboration_ns: node.self_elaboration_ns(),
            self_elaboration_heartbeats: node.self_elaboration_heartbeats(),
            content: node.content,
            position: node.position,
            end_position: node.end_position,
            edges: node.edges,
            effect_kind,
            forwards,
            backwards,
            bindings,
            diagnostics,
        })
    }

    pub fn has_effects(&self) -> bool {
        !self.backwards.is_empty()
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

    /// Check if this node's source range contains the cursor position.
    pub fn contains_cursor(&self, cursor: LineCharacterPosition) -> bool {
        range_contains_cursor(self.position, self.end_position, cursor)
    }

    /// Compute rough size of this node's range.
    pub fn range_size(&self) -> u64 {
        range_size(self.position, self.end_position)
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
    fn hyp_count(&self) -> usize {
        self.visible_bindings().len()
    }

    fn goal_count(&self) -> usize {
        1 // Always have the expression line as a goal
    }

    fn has_selectable_goals(&self) -> bool {
        true
    }
}

impl EffectFlowNode {
    pub fn visible_bindings(&self) -> Vec<&LocalBinding> {
        self.bindings
            .iter()
            .filter(|b| !b.is_implicit && !b.is_instance)
            .collect()
    }

    pub fn build_content(
        &self,
        selection: Option<Selection>,
        is_current: bool,
        timing_display: TimingDisplay,
    ) -> NodeContent {
        let title = if self.has_error() {
            " ⚠ ".to_string()
        } else {
            " ".to_string()
        };

        let mut items = self.build_binding_items(selection);
        items.push(self.build_expression_item(is_current));
        items.extend(self.build_effect_items());
        items.extend(self.build_completed_item());

        let errors = self.build_error_items();

        NodeContent {
            title,
            items,
            errors,
            self_elaboration_ns: self.self_elaboration_ns,
            self_elaboration_heartbeats: self.self_elaboration_heartbeats,
            timing_display,
        }
    }

    fn build_binding_items(&self, selection: Option<Selection>) -> Vec<ContentItem> {
        self.visible_bindings()
            .into_iter()
            .enumerate()
            .map(|(idx, b)| {
                let selected = matches!(
                    selection,
                    Some(Selection::Hyp { node_id, hyp_idx }) if node_id == self.id && hyp_idx == idx
                );
                let underline = if selected { Modifier::UNDERLINED } else { Modifier::empty() };
                let name_style = Style::new().fg(Theme::HYP_NAME).add_modifier(underline);
                let type_style = Style::new().fg(Color::White).add_modifier(underline);
                ContentItem::Row {
                    row: AlignedRow::with_styled_type(
                        b.name.clone(), name_style, ":", Style::new().fg(Color::DarkGray),
                        b.type_.to_spans(type_style),
                    ),
                    selection: Some(Selection::Hyp { node_id: self.id, hyp_idx: idx }),
                }
            })
            .collect()
    }

    fn build_expression_item(&self, is_current: bool) -> ContentItem {
        let expr_style = if self.has_error() {
            Style::new().fg(Red::BRIGHT).add_modifier(Modifier::ITALIC)
        } else if is_current {
            Style::new().fg(Color::White).add_modifier(Modifier::ITALIC)
        } else {
            Style::new().fg(Color::Gray).add_modifier(Modifier::ITALIC)
        };
        ContentItem::Line {
            spans: vec![Span::styled(
                expression_label(&self.content.to_plain_text()),
                expr_style,
            )],
            selection: Some(Selection::Goal {
                node_id: self.id,
                goal_idx: 0,
            }),
        }
    }

    fn build_effect_items(&self) -> Vec<ContentItem> {
        let mut items = Vec::new();
        for fwd in &self.forwards {
            items.push(ContentItem::Row {
                row: AlignedRow::new(
                    "",
                    Style::default(),
                    "→",
                    Style::new().fg(Color::Green),
                    fwd.value.clone(),
                    Style::new().fg(Color::White),
                ),
                selection: None,
            });
        }
        for bwd in &self.backwards {
            items.push(ContentItem::Row {
                row: AlignedRow::new(
                    "",
                    Style::default(),
                    bwd.kind.icon(),
                    Style::new().fg(Color::Magenta),
                    bwd.value.clone(),
                    Style::new().fg(Color::White),
                ),
                selection: None,
            });
        }
        items
    }

    fn build_completed_item(&self) -> Option<ContentItem> {
        if !self.is_leaf() || self.has_error() || self.has_effects() {
            return None;
        }
        let s = Style::new()
            .fg(Theme::COMPLETED_GOAL_FG)
            .add_modifier(Modifier::BOLD);
        Some(ContentItem::Row {
            row: AlignedRow::new("", Style::default(), "✓", s, "completed", s),
            selection: None,
        })
    }

    fn build_error_items(&self) -> Vec<ErrorLine> {
        let errors: Vec<ErrorLine> = self
            .diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .flat_map(|d| format_diagnostic_error_lines(&d.message))
            .collect();
        assign_error_selections(self.id, errors)
    }

    pub fn build_node_style(&self, is_current: bool) -> NodeStyle {
        let state = if self.has_error() {
            NodeVisualState::Error
        } else if is_current {
            NodeVisualState::Current
        } else if self.is_leaf() {
            NodeVisualState::CompletedLeaf
        } else {
            NodeVisualState::Inactive
        };
        let mut style = NodeStyle::from(state);
        style.title_color = style.border_color;
        style
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

const MAX_EXPRESSION_LABEL_CHARS: usize = 48;

fn expression_label(expression_source: &str) -> String {
    let single_line_expression = expression_source
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if single_line_expression.chars().count() <= MAX_EXPRESSION_LABEL_CHARS {
        single_line_expression
    } else {
        single_line_expression
            .chars()
            .take(MAX_EXPRESSION_LABEL_CHARS - 1)
            .chain(iter::once('…'))
            .collect()
    }
}

fn format_diagnostic_error_lines(message: &str) -> Vec<ErrorLine> {
    message
        .lines()
        .enumerate()
        .map(|(i, line)| {
            let prefix = if i == 0 { "✗ " } else { "  " };
            ErrorLine {
                text: format!("{prefix}{line}"),
                style: Style::new().fg(Red::BRIGHT),
                selection: None,
            }
        })
        .collect()
}

fn assign_error_selections(node_id: NodeId, mut errors: Vec<ErrorLine>) -> Vec<ErrorLine> {
    let mut error_idx = 0;
    for err in &mut errors {
        if err.text.starts_with("✗ ") {
            err.selection = Some(Selection::Error { node_id, error_idx });
            error_idx += 1;
        }
    }
    errors
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

    /// Returns true if the current node changed.
    pub fn update_current_from_cursor(&mut self, cursor: LineCharacterPosition) -> bool {
        let old = self.current_node_id;

        self.current_node_id = self
            .nodes
            .iter()
            .filter(|n| n.contains_cursor(cursor))
            .min_by_key(|n| n.range_size())
            .map(|n| n.id);

        old != self.current_node_id
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

    pub fn to_layout_graph_filtered(
        &self,
        include_expand: bool,
        timing_display: TimingDisplay,
    ) -> LayoutGraph {
        let excluded_node_ids = self.expand_target_ids();
        let mut layout = LayoutGraph::new();
        let mut id_to_index = HashMap::new();

        for node in &self.nodes {
            if !include_expand && excluded_node_ids.contains(&node.id) {
                continue;
            }
            let (width, height) = node
                .build_content(None, false, timing_display)
                .layout_size();
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

        for node in &nodes {
            tracing::trace!(
                node_id = node.id,
                kind = ?node.effect_kind,
                edges = node.edges.len(),
                content = %node.content.to_plain_text(),
                "Effect flow node"
            );
        }

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

    fn node_count(&self) -> usize {
        self.len()
    }

    fn current_node(&self) -> Option<NodeId> {
        self.current_node_id
    }
}
