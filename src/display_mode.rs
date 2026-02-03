//! Display mode management - ties together all display styles.

use std::mem::take;

use ratatui::{layout::Rect, Frame};

// Re-export mode input types for convenient access
pub use crate::before_after::BeforeAfterModeInput;
use crate::{
    before_after::BeforeAfterMode,
    data_flow::DataFlowMode,
    deduction_tree::{CompleteProofDag, ProofDagConversionError, SemanticTableau},
    effect_flow::{EffectFlowConversionError, EffectFlowDag, EffectFlowMode},
    plain_goal::PlainList,
    protocol::GenericDag,
    shared::{
        legend::LegendSection, FilterToggle, HypothesisFilters, InteractiveComponent,
        KeyMouseEvent, Selection,
    },
    tactic_tree::TacticTree,
};
pub use crate::{
    data_flow::DataFlowModeInput, deduction_tree::DeductionTreeModeInput,
    effect_flow::EffectFlowModeInput, plain_goal::PlainListInput, tactic_tree::StepsModeInput,
};

/// Trait for display modes in the TUI.
pub trait Mode: InteractiveComponent<Input = Self::Model, Event = KeyMouseEvent> {
    type Model;

    const NAME: &'static str;
    const KEYBINDINGS: &'static [(&'static str, &'static str)];
    const SUPPORTED_FILTERS: &'static [FilterToggle];

    fn current_selection(&self) -> Option<Selection>;

    /// Get legend sections for this mode.
    fn legend_sections(&self) -> Vec<LegendSection> {
        Vec::new()
    }
}

/// DAG state - either proof mode, dataflow mode, or effectflow mode.
#[derive(Clone, Default)]
pub enum DagState {
    #[default]
    None,
    Proof(CompleteProofDag),
    Dataflow(GenericDag),
    Effectflow(EffectFlowDag),
}

impl DagState {
    /// Check if any DAG is present.
    pub const fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Get the display style name of the current DAG.
    pub const fn style_name(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Proof(_) => Some("proof"),
            Self::Dataflow(_) => Some("dataflow"),
            Self::Effectflow(_) => Some("effectflow"),
        }
    }

    /// Get proof DAG if in proof mode.
    pub const fn proof(&self) -> Option<&CompleteProofDag> {
        match self {
            Self::Proof(dag) => Some(dag),
            _ => None,
        }
    }

    /// Get generic DAG if in dataflow mode.
    pub const fn dataflow(&self) -> Option<&GenericDag> {
        match self {
            Self::Dataflow(dag) => Some(dag),
            _ => None,
        }
    }

    /// Get effect flow DAG if in effectflow mode.
    pub const fn effectflow(&self) -> Option<&EffectFlowDag> {
        match self {
            Self::Effectflow(dag) => Some(dag),
            _ => None,
        }
    }

    pub fn from_generic(dag: GenericDag) -> Result<Self, DagConversionError> {
        use crate::protocol::DisplayStyle;

        match dag.display_style {
            DisplayStyle::Proof => CompleteProofDag::try_from(dag)
                .map(Self::Proof)
                .map_err(DagConversionError::Proof),
            DisplayStyle::Dataflow => Ok(Self::Dataflow(dag)),
            DisplayStyle::Effectflow => EffectFlowDag::try_from(dag)
                .map(Self::Effectflow)
                .map_err(DagConversionError::Effectflow),
        }
    }
}

/// Error when converting a `GenericDag` to a specific DAG type.
#[derive(Debug)]
pub enum DagConversionError {
    Proof(ProofDagConversionError),
    Effectflow(EffectFlowConversionError),
}

impl std::fmt::Display for DagConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Proof(e) => write!(f, "{e}"),
            Self::Effectflow(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DagConversionError {}

/// Mode variant - the actual display mode implementation.
enum ModeVariant {
    PlainGoal(PlainList),
    BeforeAfter(BeforeAfterMode),
    TacticTree(TacticTree),
    DeductionTree(SemanticTableau),
    DataFlow(DataFlowMode),
    EffectFlow(EffectFlowMode),
}

impl Default for ModeVariant {
    fn default() -> Self {
        Self::DeductionTree(SemanticTableau::default())
    }
}

/// Display mode with embedded state and shared DAG.
pub struct DisplayMode {
    /// The active mode variant.
    mode: ModeVariant,
    /// Shared DAG state (proof or dataflow).
    pub dag: DagState,
}

impl Default for DisplayMode {
    fn default() -> Self {
        Self {
            mode: ModeVariant::default(),
            dag: DagState::None,
        }
    }
}

impl DisplayMode {
    /// Get an error message if the current mode is incompatible with the DAG.
    /// Returns None if compatible or no DAG is present.
    pub fn incompatibility_error(&self) -> Option<String> {
        let dag_style = self.dag.style_name()?;

        let expected_style = match &self.mode {
            ModeVariant::PlainGoal(_)
            | ModeVariant::BeforeAfter(_)
            | ModeVariant::TacticTree(_)
            | ModeVariant::DeductionTree(_) => "proof",
            ModeVariant::DataFlow(_) => "dataflow",
            ModeVariant::EffectFlow(_) => "effectflow",
        };

        if dag_style != expected_style {
            Some(format!(
                "Mode '{}' expects '{}' DAG, but received '{}' DAG",
                self.name(),
                expected_style,
                dag_style
            ))
        } else {
            None
        }
    }

    /// Cycle to the next display mode, preserving DAG state.
    pub fn next(&mut self) {
        self.mode = match take(&mut self.mode) {
            ModeVariant::PlainGoal(_) => ModeVariant::BeforeAfter(BeforeAfterMode::default()),
            ModeVariant::BeforeAfter(_) => ModeVariant::TacticTree(TacticTree::default()),
            ModeVariant::TacticTree(_) => ModeVariant::DeductionTree(SemanticTableau::default()),
            ModeVariant::DeductionTree(_) => ModeVariant::DataFlow(DataFlowMode::default()),
            ModeVariant::DataFlow(_) => ModeVariant::EffectFlow(EffectFlowMode::default()),
            ModeVariant::EffectFlow(_) => ModeVariant::PlainGoal(PlainList::default()),
        };
    }

    /// Cycle to the previous display mode, preserving DAG state.
    pub fn prev(&mut self) {
        self.mode = match take(&mut self.mode) {
            ModeVariant::PlainGoal(_) => ModeVariant::EffectFlow(EffectFlowMode::default()),
            ModeVariant::BeforeAfter(_) => ModeVariant::PlainGoal(PlainList::default()),
            ModeVariant::TacticTree(_) => ModeVariant::BeforeAfter(BeforeAfterMode::default()),
            ModeVariant::DeductionTree(_) => ModeVariant::TacticTree(TacticTree::default()),
            ModeVariant::DataFlow(_) => ModeVariant::DeductionTree(SemanticTableau::default()),
            ModeVariant::EffectFlow(_) => ModeVariant::DataFlow(DataFlowMode::default()),
        };
    }

    /// Get the display name of the current mode.
    pub fn name(&self) -> &'static str {
        match &self.mode {
            ModeVariant::PlainGoal(_) => PlainList::NAME,
            ModeVariant::BeforeAfter(_) => BeforeAfterMode::NAME,
            ModeVariant::TacticTree(_) => TacticTree::NAME,
            ModeVariant::DeductionTree(_) => SemanticTableau::NAME,
            ModeVariant::DataFlow(_) => DataFlowMode::NAME,
            ModeVariant::EffectFlow(_) => EffectFlowMode::NAME,
        }
    }

    /// Get mode-specific keybindings.
    pub const fn keybindings(&self) -> &'static [(&'static str, &'static str)] {
        match &self.mode {
            ModeVariant::PlainGoal(_) => PlainList::KEYBINDINGS,
            ModeVariant::BeforeAfter(_) => BeforeAfterMode::KEYBINDINGS,
            ModeVariant::TacticTree(_) => TacticTree::KEYBINDINGS,
            ModeVariant::DeductionTree(_) => SemanticTableau::KEYBINDINGS,
            ModeVariant::DataFlow(_) => DataFlowMode::KEYBINDINGS,
            ModeVariant::EffectFlow(_) => EffectFlowMode::KEYBINDINGS,
        }
    }

    /// Get supported filter toggles.
    pub const fn supported_filters(&self) -> &'static [FilterToggle] {
        match &self.mode {
            ModeVariant::PlainGoal(_) => PlainList::SUPPORTED_FILTERS,
            ModeVariant::BeforeAfter(_) => BeforeAfterMode::SUPPORTED_FILTERS,
            ModeVariant::TacticTree(_) => TacticTree::SUPPORTED_FILTERS,
            ModeVariant::DeductionTree(_) => SemanticTableau::SUPPORTED_FILTERS,
            ModeVariant::DataFlow(_) => DataFlowMode::SUPPORTED_FILTERS,
            ModeVariant::EffectFlow(_) => EffectFlowMode::SUPPORTED_FILTERS,
        }
    }

    /// Get current selection from active mode.
    pub fn current_selection(&self) -> Option<Selection> {
        match &self.mode {
            ModeVariant::PlainGoal(m) => m.current_selection(),
            ModeVariant::BeforeAfter(m) => m.current_selection(),
            ModeVariant::TacticTree(m) => m.current_selection(),
            ModeVariant::DeductionTree(m) => m.current_selection(),
            ModeVariant::DataFlow(m) => m.current_selection(),
            ModeVariant::EffectFlow(m) => m.current_selection(),
        }
    }

    /// Get legend sections from active mode.
    pub fn legend_sections(&self) -> Vec<LegendSection> {
        match &self.mode {
            ModeVariant::PlainGoal(m) => m.legend_sections(),
            ModeVariant::BeforeAfter(m) => m.legend_sections(),
            ModeVariant::TacticTree(m) => m.legend_sections(),
            ModeVariant::DeductionTree(m) => m.legend_sections(),
            ModeVariant::DataFlow(m) => m.legend_sections(),
            ModeVariant::EffectFlow(m) => m.legend_sections(),
        }
    }

    /// Get filters from active mode.
    pub const fn filters(&self) -> HypothesisFilters {
        match &self.mode {
            ModeVariant::PlainGoal(m) => m.filters(),
            ModeVariant::BeforeAfter(m) => m.filters(),
            ModeVariant::TacticTree(m) => m.filters(),
            ModeVariant::DeductionTree(m) => m.filters(),
            ModeVariant::DataFlow(m) => m.filters(),
            ModeVariant::EffectFlow(m) => m.filters(),
        }
    }

    /// Whether to show the previous column (`BeforeAfter` mode only).
    pub const fn show_previous(&self) -> bool {
        match &self.mode {
            ModeVariant::BeforeAfter(m) => m.show_previous(),
            _ => false,
        }
    }

    /// Whether to show the next column (`BeforeAfter` mode only).
    pub const fn show_next(&self) -> bool {
        match &self.mode {
            ModeVariant::BeforeAfter(m) => m.show_next(),
            _ => false,
        }
    }

    /// Handle event for active mode.
    pub fn handle_event(&mut self, event: KeyMouseEvent) -> bool {
        match &mut self.mode {
            ModeVariant::PlainGoal(m) => m.handle_event(event),
            ModeVariant::BeforeAfter(m) => m.handle_event(event),
            ModeVariant::TacticTree(m) => m.handle_event(event),
            ModeVariant::DeductionTree(m) => m.handle_event(event),
            ModeVariant::DataFlow(m) => m.handle_event(event),
            ModeVariant::EffectFlow(m) => m.handle_event(event),
        }
    }

    /// Update active mode with new input data.
    pub fn update(&mut self, input: ModeInput) {
        match (&mut self.mode, input) {
            (ModeVariant::PlainGoal(m), ModeInput::PlainGoal(i)) => m.update(i),
            (ModeVariant::BeforeAfter(m), ModeInput::BeforeAfter(i)) => m.update(i),
            (ModeVariant::TacticTree(m), ModeInput::TacticTree(i)) => m.update(i),
            (ModeVariant::DeductionTree(m), ModeInput::DeductionTree(i)) => m.update(i),
            (ModeVariant::DataFlow(m), ModeInput::DataFlow(i)) => m.update(i),
            (ModeVariant::EffectFlow(m), ModeInput::EffectFlow(i)) => m.update(i),
            _ => {}
        }
    }

    /// Render active mode.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.mode {
            ModeVariant::PlainGoal(m) => m.render(frame, area),
            ModeVariant::BeforeAfter(m) => m.render(frame, area),
            ModeVariant::TacticTree(m) => m.render(frame, area),
            ModeVariant::DeductionTree(m) => m.render(frame, area),
            ModeVariant::DataFlow(m) => m.render(frame, area),
            ModeVariant::EffectFlow(m) => m.render(frame, area),
        }
    }

    /// Check if current mode is plain goal view.
    pub const fn is_plain_goal(&self) -> bool {
        matches!(self.mode, ModeVariant::PlainGoal(_))
    }

    /// Check if current mode is before/after view.
    pub const fn is_before_after(&self) -> bool {
        matches!(self.mode, ModeVariant::BeforeAfter(_))
    }

    /// Check if current mode is tactic tree view.
    pub const fn is_tactic_tree(&self) -> bool {
        matches!(self.mode, ModeVariant::TacticTree(_))
    }

    /// Check if current mode is deduction tree view.
    pub const fn is_deduction_tree(&self) -> bool {
        matches!(self.mode, ModeVariant::DeductionTree(_))
    }

    /// Check if current mode is data flow view.
    pub const fn is_data_flow(&self) -> bool {
        matches!(self.mode, ModeVariant::DataFlow(_))
    }

    /// Check if current mode is effect flow view.
    pub const fn is_effect_flow(&self) -> bool {
        matches!(self.mode, ModeVariant::EffectFlow(_))
    }
}

/// Input types for different display modes.
pub enum ModeInput {
    PlainGoal(PlainListInput),
    BeforeAfter(BeforeAfterModeInput),
    TacticTree(StepsModeInput),
    DeductionTree(DeductionTreeModeInput),
    DataFlow(DataFlowModeInput),
    EffectFlow(EffectFlowModeInput),
}
