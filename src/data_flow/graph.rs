use std::{collections::HashMap, error::Error, fmt, num::NonZero, ops::Deref};

use petgraph::graph::NodeIndex;
use ratatui::style::{Color, Modifier, Style};

use crate::{
    graph_layout::{Dag, DagGraph, DagNode, EdgeData, LayoutGraph, NavNode},
    protocol::{
        DiagnosticInfo, DiagnosticSeverity, DisplayStyle, GenericDag, GraphNode,
        LineCharacterPosition, LocalBinding, NodeId, RichText, TimingDisplay,
    },
    selection::Selection,
    theme::{Red, Theme},
    widgets::{
        aligned_table::AlignedRow,
        diff_text::RichTextExt,
        node_content::{ContentItem, ErrorLine, NodeContent, NodeStyle, NodeVisualState},
    },
};

fn parse_bindings(metadata: &serde_json::Map<String, serde_json::Value>) -> Vec<LocalBinding> {
    metadata
        .get("bindings")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default()
}

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
            Self::ConditionPart => "if",
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

    /// Get bindings with diff information (added/removed/changed).
    pub fn bindings(&self) -> Vec<LocalBinding> {
        parse_bindings(&self.inner.metadata)
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

    pub fn visible_bindings(&self) -> Vec<LocalBinding> {
        self.bindings()
            .into_iter()
            .filter(|b| !b.is_implicit && !b.is_instance)
            .collect()
    }

    pub fn build_content(
        &self,
        selection: Option<Selection>,
        _is_current: bool,
        top_down: bool,
        timing_display: TimingDisplay,
    ) -> NodeContent {
        let kind = self.data_flow_kind();
        let icon = kind.icon();
        let error_prefix = if self.has_error() { "⚠ " } else { "" };
        let expr_text = self.inner.content.to_plain_text();
        let title = match self.inner.edges.len() {
            n if n > 1 => format!(" {error_prefix}{icon} {expr_text} [{n}→] "),
            _ => format!(" {error_prefix}{icon} {expr_text} "),
        };

        let binding_items = self.build_binding_items(selection);
        let type_item = self.build_type_item(selection);

        let items = if top_down {
            let mut v = binding_items;
            v.push(type_item);
            v
        } else {
            let mut v = vec![type_item];
            v.extend(binding_items);
            v
        };

        NodeContent {
            title,
            items,
            errors: self.build_error_items(),
            self_elaboration_ns: self.inner.self_elaboration_ns(),
            self_elaboration_heartbeats: self.inner.self_elaboration_heartbeats(),
            timing_display,
        }
    }

    fn build_binding_items(&self, selection: Option<Selection>) -> Vec<ContentItem> {
        self.visible_bindings()
            .iter()
            .enumerate()
            .map(|(idx, b)| {
                let selected = matches!(
                    selection,
                    Some(Selection::Hyp { node_id, hyp_idx }) if node_id == self.id() && hyp_idx == idx
                );
                let underline = if selected { Modifier::UNDERLINED } else { Modifier::empty() };
                let name_style = Style::new().fg(Theme::HYP_NAME).add_modifier(underline);
                let type_style = Style::new().fg(Color::White).add_modifier(underline);
                ContentItem::Row {
                    row: AlignedRow::with_styled_type(
                        b.name.clone(),
                        name_style,
                        ":",
                        Style::new().fg(Color::DarkGray),
                        b.type_.to_spans(type_style),
                    ),
                    selection: Some(Selection::Hyp { node_id: self.id(), hyp_idx: idx }),
                }
            })
            .collect()
    }

    fn build_type_item(&self, selection: Option<Selection>) -> ContentItem {
        let goal_selected = matches!(
            selection,
            Some(Selection::Goal { node_id, goal_idx: 0 }) if node_id == self.id()
        );
        let underline = if goal_selected {
            Modifier::UNDERLINED
        } else {
            Modifier::empty()
        };
        let goal_sel = Some(Selection::Goal {
            node_id: self.id(),
            goal_idx: 0,
        });

        match (self.expected_type(), self.is_complete()) {
            (Some(ty), _) => ContentItem::Row {
                row: AlignedRow::new(
                    "",
                    Style::default(),
                    "⊢",
                    Style::new().fg(Color::DarkGray),
                    ty,
                    Style::new().fg(Theme::GOAL).add_modifier(underline),
                ),
                selection: goal_sel,
            },
            (None, true) => {
                let s = Style::new()
                    .fg(Theme::COMPLETED_GOAL_FG)
                    .add_modifier(Modifier::BOLD);
                ContentItem::Row {
                    row: AlignedRow::new("", Style::default(), "✓", s, "Complete", s),
                    selection: goal_sel,
                }
            }
            (None, false) => ContentItem::Row {
                row: AlignedRow::new(
                    "",
                    Style::default(),
                    "⊢",
                    Style::new().fg(Color::DarkGray),
                    "...",
                    Style::new().fg(Theme::GOAL).add_modifier(underline),
                ),
                selection: goal_sel,
            },
        }
    }

    fn build_error_items(&self) -> Vec<ErrorLine> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .enumerate()
            .map(|(error_idx, d)| {
                let msg = d.message.lines().next().unwrap_or(&d.message).to_string();
                ErrorLine {
                    text: msg,
                    style: Style::new().fg(Red::BRIGHT),
                    selection: Some(Selection::Error {
                        node_id: self.id(),
                        error_idx,
                    }),
                }
            })
            .collect()
    }

    pub fn build_node_style(&self, is_current: bool) -> NodeStyle {
        let state = if self.has_error() {
            NodeVisualState::Error
        } else if is_current {
            NodeVisualState::Current
        } else if self.is_leaf() {
            if self.is_complete() {
                NodeVisualState::CompletedLeaf
            } else {
                NodeVisualState::IncompleteLeaf
            }
        } else {
            NodeVisualState::Inactive
        };
        NodeStyle::from(state)
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
    fn hyp_count(&self) -> usize {
        self.bindings().len()
    }

    fn goal_count(&self) -> usize {
        usize::from(self.expected_type().is_some())
    }

    fn has_selectable_goals(&self) -> bool {
        self.expected_type().is_some()
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

    /// Returns true if the current node changed.
    pub fn update_current_from_cursor(&mut self, cursor: LineCharacterPosition) -> bool {
        let old = self.current_node_id;

        self.current_node_id = self
            .nodes
            .iter()
            .filter(|n| n.contains_cursor(cursor))
            .min_by_key(|n| n.range_size())
            .map(DagNode::id);

        old != self.current_node_id
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

    pub fn to_layout_graph(&self, timing_display: TimingDisplay) -> LayoutGraph {
        let mut layout = LayoutGraph::new();
        let mut id_to_index = HashMap::new();

        for node in &self.nodes {
            let (width, height) = node
                .build_content(None, false, true, timing_display)
                .layout_size();
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
                .and_then(|node| node.bindings().get(hyp_idx).cloned())
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
