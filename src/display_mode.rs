//! Display mode management - ties together all display styles.

use std::{error::Error, fmt, mem::take};

use ratatui::{layout::Rect, Frame};

use crate::{
    app::DefinitionInfo,
    before_after::{BeforeAfterMode, BeforeAfterModeInput},
    data_flow::{DataFlowConversionError, DataFlowDag, DataFlowMode, DataFlowModeInput},
    deduction_tree::{
        CompleteProofDag, DeductionTreeModeInput, ProofDagConversionError, SemanticTableau,
    },
    effect_flow::{EffectFlowConversionError, EffectFlowDag, EffectFlowMode, EffectFlowModeInput},
    plain_goal::{PlainList, PlainListInput},
    protocol::GenericDag,
    shared::{
        legend::LegendSection, FilterToggle, HypothesisFilters, InteractiveComponent,
        KeyMouseEvent, Selection,
    },
    tactic_tree::{StepsModeInput, TacticTree},
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
    Dataflow(DataFlowDag),
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

    pub fn from_generic(dag: GenericDag) -> Result<Self, DagConversionError> {
        use crate::protocol::DisplayStyle;

        match dag.display_style {
            DisplayStyle::Proof => CompleteProofDag::try_from(dag)
                .map(Self::Proof)
                .map_err(DagConversionError::Proof),
            DisplayStyle::Dataflow => DataFlowDag::try_from(dag)
                .map(Self::Dataflow)
                .map_err(DagConversionError::Dataflow),
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
    Dataflow(DataFlowConversionError),
    Effectflow(EffectFlowConversionError),
}

impl fmt::Display for DagConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proof(e) => write!(f, "{e}"),
            Self::Dataflow(e) => write!(f, "{e}"),
            Self::Effectflow(e) => write!(f, "{e}"),
        }
    }
}

impl Error for DagConversionError {}

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
    /// Shared DAG state (proof, dataflow, or effectflow).
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
    /// Switch to a mode compatible with the given DAG state.
    /// Only switches if current mode is incompatible.
    pub fn switch_to_compatible_mode(&mut self, dag: &DagState) {
        let is_compatible = match (&self.mode, dag) {
            (ModeVariant::DataFlow(_), DagState::Dataflow(_)) => true,
            (ModeVariant::EffectFlow(_), DagState::Effectflow(_)) => true,
            (
                ModeVariant::PlainGoal(_)
                | ModeVariant::BeforeAfter(_)
                | ModeVariant::TacticTree(_)
                | ModeVariant::DeductionTree(_),
                DagState::Proof(_),
            ) => true,
            (_, DagState::None) => true,
            _ => false,
        };

        if !is_compatible {
            self.mode = match dag {
                DagState::Proof(_) => ModeVariant::DeductionTree(SemanticTableau::default()),
                DagState::Dataflow(_) => ModeVariant::DataFlow(DataFlowMode::default()),
                DagState::Effectflow(_) => ModeVariant::EffectFlow(EffectFlowMode::default()),
                DagState::None => return,
            };
        }
    }

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

        if dag_style == expected_style {
            None
        } else {
            Some(format!(
                "Mode '{}' expects '{}' DAG, but received '{}' DAG",
                self.name(),
                expected_style,
                dag_style
            ))
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

    /// Take clicked error from modes that support it (if any).
    pub fn take_clicked_error(&mut self) -> Option<String> {
        match &mut self.mode {
            ModeVariant::PlainGoal(m) => m.take_clicked_error(),
            ModeVariant::BeforeAfter(m) => m.take_clicked_error(),
            ModeVariant::TacticTree(m) => m.take_clicked_error(),
            ModeVariant::DataFlow(m) => m.take_clicked_error(),
            _ => None,
        }
    }

    /// Update active mode. Each mode builds its own input from the DAG.
    #[allow(clippy::too_many_lines)]
    pub fn update(&mut self, definition: Option<DefinitionInfo>, error: Option<String>) {
        let error = error.or_else(|| self.incompatibility_error());

        match (&mut self.mode, &self.dag) {
            (ModeVariant::PlainGoal(m), DagState::Proof(dag)) => {
                m.update(
                    PlainListInput::builder()
                        .dag(dag)
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }
            (ModeVariant::PlainGoal(m), _) => {
                m.update(
                    PlainListInput::builder()
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }

            (ModeVariant::BeforeAfter(m), DagState::Proof(dag)) => {
                m.update(
                    BeforeAfterModeInput::builder()
                        .dag(dag)
                        .definition(definition)
                        .error(error)
                        .show_previous(m.show_previous())
                        .show_next(m.show_next())
                        .build(),
                );
            }
            (ModeVariant::BeforeAfter(m), _) => {
                m.update(
                    BeforeAfterModeInput::builder()
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }

            (ModeVariant::TacticTree(m), DagState::Proof(dag)) => {
                m.update(
                    StepsModeInput::builder()
                        .dag(dag)
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }
            (ModeVariant::TacticTree(m), _) => {
                m.update(
                    StepsModeInput::builder()
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }

            (ModeVariant::DeductionTree(m), DagState::Proof(dag)) => {
                m.update(
                    DeductionTreeModeInput::builder()
                        .dag(dag)
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }
            (ModeVariant::DeductionTree(m), _) => {
                m.update(
                    DeductionTreeModeInput::builder()
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }

            (ModeVariant::DataFlow(m), DagState::Dataflow(dag)) => {
                m.update(
                    DataFlowModeInput::builder()
                        .dag(dag)
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }
            (ModeVariant::DataFlow(m), _) => {
                m.update(
                    DataFlowModeInput::builder()
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }

            (ModeVariant::EffectFlow(m), DagState::Effectflow(dag)) => {
                m.update(
                    EffectFlowModeInput::builder()
                        .effect_dag(dag)
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }
            (ModeVariant::EffectFlow(m), _) => {
                m.update(
                    EffectFlowModeInput::builder()
                        .definition(definition)
                        .error(error)
                        .build(),
                );
            }
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
}
