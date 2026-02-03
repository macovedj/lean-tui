use ratatui::style::Color;

use crate::{
    protocol::{GenericDag, GraphEdge, GraphNode},
    shared::Theme,
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
    /// Effect flow (monadic effect propagation)
    EffectFlow,
    /// Error propagation path
    ErrorPropagation,
    /// Early return path
    EarlyReturn,
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
            Self::Unknown => Color::DarkGray,
        }
    }

    /// Get an icon for this edge kind.
    pub const fn icon(self) -> &'static str {
        match self {
            Self::DataFlow => "→",
            Self::ControlFlow => "⇒",
            Self::EffectFlow => "⤳",
            Self::ErrorPropagation => "⚠→",
            Self::EarlyReturn => "↩",
            Self::Unknown => "?",
        }
    }
}

/// Extension trait for `GraphEdge` in effectflow mode.
pub trait EffectFlowEdgeExt {
    /// Get the edge kind.
    fn edge_kind(&self) -> EdgeKind;
}

impl EffectFlowEdgeExt for GraphEdge {
    fn edge_kind(&self) -> EdgeKind {
        self.kind
            .as_deref()
            .map_or(EdgeKind::Unknown, EdgeKind::from_str)
    }
}

/// Extension trait for `GenericDag` in effectflow mode.
pub trait EffectFlowDagExt {
    /// Get the monad stack from DAG metadata.
    fn monad_stack(&self) -> Option<Vec<String>>;
}

impl EffectFlowDagExt for GenericDag {
    fn monad_stack(&self) -> Option<Vec<String>> {
        self.metadata
            .get("monad_stack")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Extension trait for `GraphNode` in effectflow mode.
pub trait EffectFlowNodeExt {
    /// Get the effect kind of this node.
    fn effect_kind(&self) -> EffectKind;

    /// Check if this node has effect metadata (required for effect flow mode).
    fn has_effect_metadata(&self) -> bool;

    /// Get forward interpretation from node metadata.
    fn forward(&self) -> Option<String>;

    /// Get backward interpretation from node metadata.
    fn backward(&self) -> Option<String>;

    /// Get bindings read by this effect.
    fn bindings_read(&self) -> Vec<String>;

    /// Get bindings written by this effect.
    fn bindings_written(&self) -> Vec<String>;
}

impl EffectFlowNodeExt for GraphNode {
    fn effect_kind(&self) -> EffectKind {
        self.kind
            .as_deref()
            .map_or_else(|| EffectKind::Custom(String::new()), EffectKind::from_str)
    }

    fn has_effect_metadata(&self) -> bool {
        self.kind.is_some()
    }

    fn forward(&self) -> Option<String> {
        self.metadata
            .get("forward")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    fn backward(&self) -> Option<String> {
        self.metadata
            .get("backward")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    fn bindings_read(&self) -> Vec<String> {
        self.metadata
            .get("bindings_read")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    fn bindings_written(&self) -> Vec<String> {
        self.metadata
            .get("bindings_written")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }
}

// --- Validated EffectFlowDag ---

use std::{error::Error, fmt, ops::Deref};

use crate::protocol::DisplayStyle;

/// Error when converting a `GenericDag` to `EffectFlowDag`.
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

/// Validated effect flow DAG - guarantees all nodes have effect metadata.
#[derive(Debug, Clone)]
pub struct EffectFlowDag {
    /// The underlying generic DAG.
    pub inner: GenericDag,
}

impl EffectFlowDag {
    /// Get the inner generic DAG.
    pub const fn as_generic(&self) -> &GenericDag {
        &self.inner
    }

    /// Get the number of nodes.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

impl Deref for EffectFlowDag {
    type Target = GenericDag;

    fn deref(&self) -> &Self::Target {
        &self.inner
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

        // Validate that all nodes have effect kind metadata
        for node in &dag.nodes {
            if !node.has_effect_metadata() {
                return Err(EffectFlowConversionError::MissingEffectKind { node_id: node.id });
            }
        }

        Ok(Self { inner: dag })
    }
}
