//! Display modes for the TUI.

mod before_after;
mod data_flow;
pub mod deduction_tree;
mod open_goal_list;
mod steps_view;

use std::mem::take;

use before_after::BeforeAfterMode;
pub use before_after::BeforeAfterModeInput;
use data_flow::DataFlowMode;
pub use data_flow::DataFlowModeInput;
pub use deduction_tree::DeductionTreeModeInput;
use deduction_tree::SemanticTableau;
use open_goal_list::PlainList;
pub use open_goal_list::PlainListInput;
use ratatui::{layout::Rect, Frame};
pub use steps_view::StepsModeInput;
use steps_view::TacticTree;

use crate::{
    protocol::{CompleteFunctionalDag, CompleteProofDag},
    widgets::{FilterToggle, HypothesisFilters, InteractiveComponent, KeyMouseEvent, Selection},
};

/// DAG state - either proof mode or functional mode.
#[derive(Clone, Default)]
pub enum DagState {
    #[default]
    None,
    Proof(CompleteProofDag),
    Functional(CompleteFunctionalDag),
}

impl DagState {
    /// Check if any DAG is present.
    pub const fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Get proof DAG if in proof mode.
    pub const fn proof(&self) -> Option<&CompleteProofDag> {
        match self {
            Self::Proof(dag) => Some(dag),
            _ => None,
        }
    }

    /// Get functional DAG if in functional mode.
    pub const fn functional(&self) -> Option<&CompleteFunctionalDag> {
        match self {
            Self::Functional(dag) => Some(dag),
            _ => None,
        }
    }
}

/// Trait for display modes in the TUI.
pub trait Mode: InteractiveComponent<Input = Self::Model, Event = KeyMouseEvent> {
    type Model;

    const NAME: &'static str;
    const KEYBINDINGS: &'static [(&'static str, &'static str)];
    const SUPPORTED_FILTERS: &'static [FilterToggle];

    fn current_selection(&self) -> Option<Selection>;
}

/// Mode variant - the actual display mode implementation.
enum ModeVariant {
    OpenGoalList(PlainList),
    BeforeAfter(BeforeAfterMode),
    StepsView(TacticTree),
    DeductionTree(SemanticTableau),
    DataFlow(DataFlowMode),
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
    /// Shared DAG state (proof or functional).
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
    /// Cycle to the next display mode, preserving DAG state.
    pub fn next(&mut self) {
        self.mode = match take(&mut self.mode) {
            ModeVariant::OpenGoalList(_) => ModeVariant::BeforeAfter(BeforeAfterMode::default()),
            ModeVariant::BeforeAfter(_) => ModeVariant::StepsView(TacticTree::default()),
            ModeVariant::StepsView(_) => ModeVariant::DeductionTree(SemanticTableau::default()),
            ModeVariant::DeductionTree(_) => ModeVariant::DataFlow(DataFlowMode::default()),
            ModeVariant::DataFlow(_) => ModeVariant::OpenGoalList(PlainList::default()),
        };
    }

    /// Cycle to the previous display mode, preserving DAG state.
    pub fn prev(&mut self) {
        self.mode = match take(&mut self.mode) {
            ModeVariant::OpenGoalList(_) => ModeVariant::DataFlow(DataFlowMode::default()),
            ModeVariant::BeforeAfter(_) => ModeVariant::OpenGoalList(PlainList::default()),
            ModeVariant::StepsView(_) => ModeVariant::BeforeAfter(BeforeAfterMode::default()),
            ModeVariant::DeductionTree(_) => ModeVariant::StepsView(TacticTree::default()),
            ModeVariant::DataFlow(_) => ModeVariant::DeductionTree(SemanticTableau::default()),
        };
    }

    /// Get the display name of the current mode.
    pub fn name(&self) -> &'static str {
        match &self.mode {
            ModeVariant::OpenGoalList(_) => PlainList::NAME,
            ModeVariant::BeforeAfter(_) => BeforeAfterMode::NAME,
            ModeVariant::StepsView(_) => TacticTree::NAME,
            ModeVariant::DeductionTree(_) => SemanticTableau::NAME,
            ModeVariant::DataFlow(_) => DataFlowMode::NAME,
        }
    }

    /// Get mode-specific keybindings.
    pub const fn keybindings(&self) -> &'static [(&'static str, &'static str)] {
        match &self.mode {
            ModeVariant::OpenGoalList(_) => PlainList::KEYBINDINGS,
            ModeVariant::BeforeAfter(_) => BeforeAfterMode::KEYBINDINGS,
            ModeVariant::StepsView(_) => TacticTree::KEYBINDINGS,
            ModeVariant::DeductionTree(_) => SemanticTableau::KEYBINDINGS,
            ModeVariant::DataFlow(_) => DataFlowMode::KEYBINDINGS,
        }
    }

    /// Get supported filter toggles.
    pub const fn supported_filters(&self) -> &'static [FilterToggle] {
        match &self.mode {
            ModeVariant::OpenGoalList(_) => PlainList::SUPPORTED_FILTERS,
            ModeVariant::BeforeAfter(_) => BeforeAfterMode::SUPPORTED_FILTERS,
            ModeVariant::StepsView(_) => TacticTree::SUPPORTED_FILTERS,
            ModeVariant::DeductionTree(_) => SemanticTableau::SUPPORTED_FILTERS,
            ModeVariant::DataFlow(_) => DataFlowMode::SUPPORTED_FILTERS,
        }
    }

    /// Get current selection from active mode.
    pub fn current_selection(&self) -> Option<Selection> {
        match &self.mode {
            ModeVariant::OpenGoalList(m) => m.current_selection(),
            ModeVariant::BeforeAfter(m) => m.current_selection(),
            ModeVariant::StepsView(m) => m.current_selection(),
            ModeVariant::DeductionTree(m) => m.current_selection(),
            ModeVariant::DataFlow(m) => m.current_selection(),
        }
    }

    /// Get filters from active mode.
    pub const fn filters(&self) -> HypothesisFilters {
        match &self.mode {
            ModeVariant::OpenGoalList(m) => m.filters(),
            ModeVariant::BeforeAfter(m) => m.filters(),
            ModeVariant::StepsView(m) => m.filters(),
            ModeVariant::DeductionTree(m) => m.filters(),
            ModeVariant::DataFlow(m) => m.filters(),
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
            ModeVariant::OpenGoalList(m) => m.handle_event(event),
            ModeVariant::BeforeAfter(m) => m.handle_event(event),
            ModeVariant::StepsView(m) => m.handle_event(event),
            ModeVariant::DeductionTree(m) => m.handle_event(event),
            ModeVariant::DataFlow(m) => m.handle_event(event),
        }
    }

    /// Render active mode.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        match &mut self.mode {
            ModeVariant::OpenGoalList(m) => m.render(frame, area),
            ModeVariant::BeforeAfter(m) => m.render(frame, area),
            ModeVariant::StepsView(m) => m.render(frame, area),
            ModeVariant::DeductionTree(m) => m.render(frame, area),
            ModeVariant::DataFlow(m) => m.render(frame, area),
        }
    }

    /// Update active mode with appropriate input.
    pub fn update_open_goal_list(&mut self, input: PlainListInput) {
        if let ModeVariant::OpenGoalList(m) = &mut self.mode {
            m.update(input);
        }
    }

    pub fn update_before_after(&mut self, input: BeforeAfterModeInput) {
        if let ModeVariant::BeforeAfter(m) = &mut self.mode {
            m.update(input);
        }
    }

    pub fn update_steps(&mut self, input: StepsModeInput) {
        if let ModeVariant::StepsView(m) = &mut self.mode {
            m.update(input);
        }
    }

    pub fn update_deduction_tree(&mut self, input: DeductionTreeModeInput) {
        if let ModeVariant::DeductionTree(m) = &mut self.mode {
            m.update(input);
        }
    }

    pub fn update_data_flow(&mut self, input: DataFlowModeInput) {
        if let ModeVariant::DataFlow(m) = &mut self.mode {
            m.update(input);
        }
    }
}
