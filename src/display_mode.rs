//! Display mode management - ties together all display styles.

use std::{error::Error, fmt, mem::take};

use ratatui::{layout::Rect, Frame};

use crate::{
    before_after::BeforeAfterMode,
    compact_tree::CompactTree,
    data_flow::{DataFlowConversionError, DataFlowDag, DataFlowMode},
    deduction_tree::{CompleteProofDag, ProofDagConversionError, SemanticTableau},
    effect_flow::{EffectFlowConversionError, EffectFlowDag, EffectFlowMode},
    interactive::{InteractiveComponent, KeyMouseEvent, ModeInput},
    protocol::{GenericDag, LocalBinding, RichText},
    selection::Selection,
    widgets::legend::LegendSection,
};

/// Trait for display modes in the TUI.
pub trait Mode: InteractiveComponent<Input = Self::Model, Event = KeyMouseEvent> {
    type Model;

    fn name(&self) -> &'static str;
    fn keybindings(&self) -> &'static [(&'static str, &'static str)];
    fn current_selection(&self) -> Option<Selection>;

    fn legend_sections(&self) -> Vec<LegendSection> {
        Vec::new()
    }

    fn take_clicked_error(&mut self) -> Option<String> {
        None
    }
}

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

    /// Get block input bindings (theorem/def parameters).
    pub fn block_input(&self) -> &[LocalBinding] {
        match self {
            Self::Proof(dag) => &dag.block_input,
            Self::Dataflow(dag) => &dag.block_input,
            Self::Effectflow(dag) => &dag.block_input,
            Self::None => &[],
        }
    }

    /// Get block output (theorem statement).
    pub fn block_output(&self) -> Option<&RichText> {
        match self {
            Self::Proof(dag) => dag.block_output.as_ref(),
            Self::Dataflow(dag) => dag.block_output.as_ref(),
            Self::Effectflow(dag) => dag.block_output.as_ref(),
            Self::None => None,
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
    Compact(BeforeAfterMode),
    CompactTree(CompactTree),
    DeductionTree(SemanticTableau),
    DataFlow(DataFlowMode),
    EffectFlow(EffectFlowMode),
}

impl Default for ModeVariant {
    fn default() -> Self {
        Self::DeductionTree(SemanticTableau::default())
    }
}

pub struct DisplayMode {
    mode: ModeVariant,
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
    pub fn switch_to_compatible_mode(&mut self, dag: &DagState) {
        let is_compatible = matches!(
            (&self.mode, dag),
            (ModeVariant::DataFlow(_), DagState::Dataflow(_))
                | (ModeVariant::EffectFlow(_), DagState::Effectflow(_))
                | (
                    ModeVariant::Compact(_)
                        | ModeVariant::CompactTree(_)
                        | ModeVariant::DeductionTree(_),
                    DagState::Proof(_)
                )
                | (_, DagState::None)
        );

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
            ModeVariant::Compact(_)
            | ModeVariant::CompactTree(_)
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
            ModeVariant::Compact(_) => ModeVariant::CompactTree(CompactTree::default()),
            ModeVariant::CompactTree(_) => ModeVariant::DeductionTree(SemanticTableau::default()),
            ModeVariant::DeductionTree(_) => ModeVariant::DataFlow(DataFlowMode::default()),
            ModeVariant::DataFlow(_) => ModeVariant::EffectFlow(EffectFlowMode::default()),
            ModeVariant::EffectFlow(_) => ModeVariant::Compact(BeforeAfterMode::default()),
        };
    }

    /// Cycle to the previous display mode, preserving DAG state.
    pub fn prev(&mut self) {
        self.mode = match take(&mut self.mode) {
            ModeVariant::Compact(_) => ModeVariant::EffectFlow(EffectFlowMode::default()),
            ModeVariant::CompactTree(_) => ModeVariant::Compact(BeforeAfterMode::default()),
            ModeVariant::DeductionTree(_) => ModeVariant::CompactTree(CompactTree::default()),
            ModeVariant::DataFlow(_) => ModeVariant::DeductionTree(SemanticTableau::default()),
            ModeVariant::EffectFlow(_) => ModeVariant::DataFlow(DataFlowMode::default()),
        };
    }

    /// Get the display name of the current mode.
    pub fn name(&self) -> &'static str {
        match &self.mode {
            ModeVariant::Compact(m) => m.name(),
            ModeVariant::CompactTree(m) => m.name(),
            ModeVariant::DeductionTree(m) => m.name(),
            ModeVariant::DataFlow(m) => m.name(),
            ModeVariant::EffectFlow(m) => m.name(),
        }
    }

    /// Get mode-specific keybindings.
    pub fn keybindings(&self) -> &'static [(&'static str, &'static str)] {
        match &self.mode {
            ModeVariant::Compact(m) => m.keybindings(),
            ModeVariant::CompactTree(m) => m.keybindings(),
            ModeVariant::DeductionTree(m) => m.keybindings(),
            ModeVariant::DataFlow(m) => m.keybindings(),
            ModeVariant::EffectFlow(m) => m.keybindings(),
        }
    }

    /// Get current selection from active mode.
    pub fn current_selection(&self) -> Option<Selection> {
        match &self.mode {
            ModeVariant::Compact(m) => m.current_selection(),
            ModeVariant::CompactTree(m) => m.current_selection(),
            ModeVariant::DeductionTree(m) => m.current_selection(),
            ModeVariant::DataFlow(m) => m.current_selection(),
            ModeVariant::EffectFlow(m) => m.current_selection(),
        }
    }

    /// Get legend sections from active mode.
    pub fn legend_sections(&self) -> Vec<LegendSection> {
        match &self.mode {
            ModeVariant::Compact(m) => m.legend_sections(),
            ModeVariant::CompactTree(m) => m.legend_sections(),
            ModeVariant::DeductionTree(m) => m.legend_sections(),
            ModeVariant::DataFlow(m) => m.legend_sections(),
            ModeVariant::EffectFlow(m) => m.legend_sections(),
        }
    }

    /// Handle event for active mode.
    pub fn handle_event(&mut self, event: KeyMouseEvent) -> bool {
        match &mut self.mode {
            ModeVariant::Compact(m) => m.handle_event(event),
            ModeVariant::CompactTree(m) => m.handle_event(event),
            ModeVariant::DeductionTree(m) => m.handle_event(event),
            ModeVariant::DataFlow(m) => m.handle_event(event),
            ModeVariant::EffectFlow(m) => m.handle_event(event),
        }
    }

    /// Take clicked error from modes that support it (if any).
    pub fn take_clicked_error(&mut self) -> Option<String> {
        match &mut self.mode {
            ModeVariant::Compact(m) => Mode::take_clicked_error(m),
            ModeVariant::CompactTree(m) => Mode::take_clicked_error(m),
            ModeVariant::DeductionTree(m) => Mode::take_clicked_error(m),
            ModeVariant::DataFlow(m) => Mode::take_clicked_error(m),
            ModeVariant::EffectFlow(m) => Mode::take_clicked_error(m),
        }
    }

    /// Update active mode. Each mode builds its own input from the DAG.
    pub fn update(&mut self, error: Option<String>) {
        let error = error.or_else(|| self.incompatibility_error());

        match (&mut self.mode, &self.dag) {
            (ModeVariant::Compact(m), DagState::Proof(dag)) => {
                m.update(ModeInput {
                    dag: Some(dag.clone()),
                    error,
                });
            }
            (ModeVariant::Compact(m), _) => {
                m.update(ModeInput { dag: None, error });
            }

            (ModeVariant::CompactTree(m), DagState::Proof(dag)) => {
                m.update(ModeInput {
                    dag: Some(dag.clone()),
                    error,
                });
            }
            (ModeVariant::CompactTree(m), _) => {
                m.update(ModeInput { dag: None, error });
            }

            (ModeVariant::DeductionTree(m), DagState::Proof(dag)) => {
                m.update(ModeInput {
                    dag: Some(dag.clone()),
                    error,
                });
            }
            (ModeVariant::DeductionTree(m), _) => {
                m.update(ModeInput { dag: None, error });
            }

            (ModeVariant::DataFlow(m), DagState::Dataflow(dag)) => {
                m.update(ModeInput {
                    dag: Some(dag.clone()),
                    error,
                });
            }
            (ModeVariant::DataFlow(m), _) => {
                m.update(ModeInput { dag: None, error });
            }

            (ModeVariant::EffectFlow(m), DagState::Effectflow(dag)) => {
                m.update(ModeInput {
                    dag: Some(dag.clone()),
                    error,
                });
            }
            (ModeVariant::EffectFlow(m), _) => {
                m.update(ModeInput { dag: None, error });
            }
        }
    }

    /// Render active mode.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.mode {
            ModeVariant::Compact(m) => m.render(frame, area),
            ModeVariant::CompactTree(m) => m.render(frame, area),
            ModeVariant::DeductionTree(m) => m.render(frame, area),
            ModeVariant::DataFlow(m) => m.render(frame, area),
            ModeVariant::EffectFlow(m) => m.render(frame, area),
        }
    }
}
