use std::{error::Error, fmt};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

use crate::{
    before_after::{
        apply_before_after_action, before_after_legend_sections, draw_before_after, BeforeAfterView,
    },
    compact_tree::{
        apply_compact_tree_action, compact_tree_legend_sections, draw_compact_tree, CompactTreeView,
    },
    data_flow::{
        apply_data_flow_action, current_data_flow_selection, data_flow_error_message,
        data_flow_legend_sections, data_flow_selectable_items, draw_data_flow,
        node_kind_legend_items, DataFlowConversionError, DataFlowDag,
    },
    deduction_tree::{
        apply_tableau_action, current_tableau_selection, draw_semantic_tableau,
        tableau_legend_sections, tableau_selectable_items, CompleteProofDag,
        ProofDagConversionError, EMPTY_PROOF_DAG,
    },
    effect_flow::{
        apply_effect_flow_action, current_effect_flow_selection, draw_effect_flow,
        effect_flow_legend_sections, effect_flow_selectable_items, EffectFlowConversionError,
        EffectFlowDag, EffectFlowView,
    },
    graph_layout::TreeGraphView,
    keybindings::{
        KeyBinding, BEFORE_AFTER_KEYBINDINGS, COMPACT_TREE_KEYBINDINGS, DATA_FLOW_KEYBINDINGS,
        EFFECT_FLOW_KEYBINDINGS, SEMANTIC_TABLEAU_KEYBINDINGS,
    },
    message::{ClickTarget, ModeAction},
    protocol::{
        DisplayStyle, FullyQualifiedDocumentPosition, GenericDag, LineCharacterPosition,
        LocalBinding, RichText, TimingDisplay,
    },
    selection::Selection,
    widgets::{legend::LegendSection, render_helpers::render_no_goals},
};

#[derive(Clone, Default)]
pub enum DagState {
    #[default]
    None,
    Proof(CompleteProofDag),
    Dataflow(DataFlowDag),
    Effectflow(EffectFlowDag),
}

impl DagState {
    pub const fn is_some(&self) -> bool {
        !matches!(self, Self::None)
    }

    pub const fn style_name(&self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Proof(_) => Some("proof"),
            Self::Dataflow(_) => Some("dataflow"),
            Self::Effectflow(_) => Some("effectflow"),
        }
    }

    pub const fn proof(&self) -> Option<&CompleteProofDag> {
        match self {
            Self::Proof(dag) => Some(dag),
            _ => None,
        }
    }

    fn proof_or_empty(&self) -> &CompleteProofDag {
        self.proof().unwrap_or(&EMPTY_PROOF_DAG)
    }

    pub fn block_input(&self) -> &[LocalBinding] {
        match self {
            Self::Proof(dag) => &dag.block_input,
            Self::Dataflow(dag) => &dag.block_input,
            Self::Effectflow(dag) => &dag.block_input,
            Self::None => &[],
        }
    }

    pub fn block_output(&self) -> Option<&RichText> {
        match self {
            Self::Proof(dag) => dag.block_output.as_ref(),
            Self::Dataflow(dag) => dag.block_output.as_ref(),
            Self::Effectflow(dag) => dag.block_output.as_ref(),
            Self::None => None,
        }
    }

    pub fn from_generic(dag: GenericDag) -> Result<Self, DagConversionError> {
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

    pub fn update_current_from_cursor(&mut self, cursor: LineCharacterPosition) -> bool {
        match self {
            Self::Proof(dag) => dag.update_current_from_cursor(cursor),
            Self::Dataflow(dag) => dag.update_current_from_cursor(cursor),
            Self::Effectflow(dag) => dag.update_current_from_cursor(cursor),
            Self::None => false,
        }
    }

    pub fn navigation_location(
        &self,
        selection: Selection,
        current_uri: &str,
    ) -> Option<FullyQualifiedDocumentPosition> {
        match self {
            Self::Proof(dag) => dag.navigation_location(selection, current_uri),
            Self::Dataflow(dag) => dag.navigation_location(selection, current_uri),
            Self::Effectflow(_) | Self::None => None,
        }
    }
}

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

#[derive(Debug, Clone, Copy)]
pub enum DisplayMode {
    BeforeAfter(BeforeAfterView),
    CompactTree(CompactTreeView),
    SemanticTableau(TreeGraphView),
    DataFlow(TreeGraphView),
    EffectFlow(EffectFlowView),
}

impl Default for DisplayMode {
    fn default() -> Self {
        Self::SemanticTableau(TreeGraphView::default())
    }
}

impl DisplayMode {
    const fn expected_style(&self) -> &'static str {
        match self {
            Self::BeforeAfter(_) | Self::CompactTree(_) | Self::SemanticTableau(_) => "proof",
            Self::DataFlow(_) => "dataflow",
            Self::EffectFlow(_) => "effectflow",
        }
    }

    fn accepts(&self, dag: &DagState) -> bool {
        dag.style_name() == Some(self.expected_style())
    }

    #[must_use]
    pub fn compatible_with(self, dag: &DagState) -> Self {
        match dag {
            DagState::None => self,
            _ if self.accepts(dag) => self,
            DagState::Proof(_) => Self::SemanticTableau(TreeGraphView::default()),
            DagState::Dataflow(_) => Self::DataFlow(TreeGraphView::default()),
            DagState::Effectflow(_) => Self::EffectFlow(EffectFlowView::default()),
        }
    }

    pub fn incompatibility_error(&self, dag: &DagState) -> Option<String> {
        let dag_style = dag.style_name()?;
        (!self.accepts(dag)).then(|| {
            format!(
                "Mode '{}' expects '{}' DAG, but received '{}' DAG",
                self.name(),
                self.expected_style(),
                dag_style
            )
        })
    }

    #[must_use]
    pub fn next(&self) -> Self {
        match self {
            Self::BeforeAfter(_) => Self::CompactTree(CompactTreeView::default()),
            Self::CompactTree(_) => Self::SemanticTableau(TreeGraphView::default()),
            Self::SemanticTableau(_) => Self::DataFlow(TreeGraphView::default()),
            Self::DataFlow(_) => Self::EffectFlow(EffectFlowView::default()),
            Self::EffectFlow(_) => Self::BeforeAfter(BeforeAfterView::default()),
        }
    }

    #[must_use]
    pub fn previous(&self) -> Self {
        match self {
            Self::BeforeAfter(_) => Self::EffectFlow(EffectFlowView::default()),
            Self::CompactTree(_) => Self::BeforeAfter(BeforeAfterView::default()),
            Self::SemanticTableau(_) => Self::CompactTree(CompactTreeView::default()),
            Self::DataFlow(_) => Self::SemanticTableau(TreeGraphView::default()),
            Self::EffectFlow(_) => Self::DataFlow(TreeGraphView::default()),
        }
    }

    pub const fn name(&self) -> &'static str {
        match self {
            Self::BeforeAfter(_) => "Compact",
            Self::CompactTree(_) => "Compact Tree",
            Self::SemanticTableau(_) => "Semantic tableau",
            Self::DataFlow(_) => "Data Flow",
            Self::EffectFlow(_) => "Effect Flow",
        }
    }

    pub const fn keybindings(&self) -> &'static [KeyBinding<ModeAction>] {
        match self {
            Self::BeforeAfter(_) => BEFORE_AFTER_KEYBINDINGS,
            Self::CompactTree(_) => COMPACT_TREE_KEYBINDINGS,
            Self::SemanticTableau(_) => SEMANTIC_TABLEAU_KEYBINDINGS,
            Self::DataFlow(_) => DATA_FLOW_KEYBINDINGS,
            Self::EffectFlow(_) => EFFECT_FLOW_KEYBINDINGS,
        }
    }

    pub fn legend_sections(&self, dag: &DagState) -> Vec<LegendSection> {
        match (self, dag) {
            (Self::BeforeAfter(_), _) => before_after_legend_sections(),
            (Self::CompactTree(_), _) => compact_tree_legend_sections(),
            (Self::SemanticTableau(_), _) => tableau_legend_sections(),
            (Self::DataFlow(_), DagState::Dataflow(dag)) => {
                data_flow_legend_sections(node_kind_legend_items(dag))
            }
            (Self::DataFlow(_), _) => data_flow_legend_sections(Vec::new()),
            (Self::EffectFlow(_), _) => effect_flow_legend_sections(),
        }
    }

    pub fn current_selection(&self, dag: &DagState) -> Option<Selection> {
        match (self, dag) {
            (
                Self::BeforeAfter(BeforeAfterView { selection, .. })
                | Self::CompactTree(CompactTreeView { selection, .. }),
                _,
            ) => selection.current(dag.proof_or_empty()),
            (Self::SemanticTableau(view), DagState::Proof(dag)) => {
                current_tableau_selection(view, dag)
            }
            (Self::DataFlow(view), DagState::Dataflow(dag)) => {
                current_data_flow_selection(view, dag)
            }
            (Self::EffectFlow(view), DagState::Effectflow(dag)) => {
                current_effect_flow_selection(view, dag)
            }
            _ => None,
        }
    }

    pub fn select(&mut self, clicked: Selection, dag: &DagState) {
        match (self, dag) {
            (
                Self::BeforeAfter(BeforeAfterView { selection, .. })
                | Self::CompactTree(CompactTreeView { selection, .. }),
                _,
            ) => selection.click(clicked, dag.proof_or_empty()),
            (Self::SemanticTableau(view), DagState::Proof(dag)) => {
                view.selection
                    .click(clicked, dag, &tableau_selectable_items(dag));
            }
            (Self::DataFlow(view), DagState::Dataflow(dag)) => {
                view.selection
                    .click(clicked, dag, &data_flow_selectable_items(dag));
            }
            (Self::EffectFlow(view), DagState::Effectflow(dag)) => {
                view.selection
                    .click(clicked, dag, &effect_flow_selectable_items(dag));
            }
            _ => {}
        }
    }

    pub fn clicked_error_message(&self, selection: Selection, dag: &DagState) -> Option<String> {
        let Selection::Error { node_id, error_idx } = selection else {
            return None;
        };
        match (self, dag) {
            (Self::SemanticTableau(_), DagState::Proof(dag)) => {
                dag.get_error_message(node_id, error_idx)
            }
            (Self::DataFlow(_), DagState::Dataflow(dag)) => {
                data_flow_error_message(dag, node_id, error_idx)
            }
            _ => None,
        }
    }

    pub fn apply_action(&mut self, action: ModeAction, dag: &DagState, timing: TimingDisplay) {
        match (self, dag) {
            (Self::BeforeAfter(view), _) => {
                apply_before_after_action(view, action, dag.proof_or_empty());
            }
            (Self::CompactTree(view), _) => {
                apply_compact_tree_action(view, action, dag.proof_or_empty());
            }
            (Self::SemanticTableau(view), DagState::Proof(dag)) => {
                apply_tableau_action(view, action, dag, timing);
            }
            (Self::DataFlow(view), DagState::Dataflow(dag)) => {
                apply_data_flow_action(view, action, dag, timing);
            }
            (Self::EffectFlow(view), DagState::Effectflow(dag)) => {
                apply_effect_flow_action(view, action, dag, timing);
            }
            (Self::SemanticTableau(view) | Self::DataFlow(view), _) => view.toggle(action),
            (Self::EffectFlow(view), _) => view.toggle(action),
        }
    }

    pub fn draw(
        &self,
        dag: &DagState,
        timing: TimingDisplay,
        area: Rect,
        buf: &mut Buffer,
    ) -> Vec<ClickTarget> {
        match (self, dag) {
            (Self::BeforeAfter(view), _) => {
                draw_before_after(view, dag.proof_or_empty(), area, buf)
            }
            (Self::CompactTree(view), _) => {
                draw_compact_tree(view, dag.proof_or_empty(), area, buf)
            }
            (Self::SemanticTableau(view), DagState::Proof(dag)) => {
                draw_semantic_tableau(view, dag, timing, area, buf)
            }
            (Self::DataFlow(view), DagState::Dataflow(dag)) => {
                draw_data_flow(view, dag, timing, area, buf)
            }
            (Self::EffectFlow(view), DagState::Effectflow(dag)) => {
                draw_effect_flow(view, dag, timing, area, buf)
            }
            (Self::SemanticTableau(_), _) => {
                render_no_goals(area, buf);
                Vec::new()
            }
            (Self::DataFlow(_), _) => {
                draw_missing_dag("No data flow available (term mode only)", area, buf)
            }
            (Self::EffectFlow(_), _) => draw_missing_dag("No effect flow DAG available", area, buf),
        }
    }
}

fn draw_missing_dag(text: &str, area: Rect, buf: &mut Buffer) -> Vec<ClickTarget> {
    Paragraph::new(text)
        .style(Style::new().fg(Color::DarkGray))
        .render(area, buf);
    Vec::new()
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::scenario_replay::{key, render_scenario, ReplayedMessages};

    #[test]
    fn data_flow_dag_in_effect_flow_mode_reports_incompatibility() {
        assert_snapshot!(render_scenario(
            "sorting",
            &ReplayedMessages::All,
            &[key(']')]
        ));
    }
}
