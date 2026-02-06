use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};
use tracing::trace;

use crate::{
    app::DefinitionInfo,
    deduction_tree::{
        layout::{SemanticTableauLayout, SemanticTableauState},
        types::CompleteProofDag,
        TacticProofState,
    },
    display_mode::Mode,
    shared::{
        find_nearest_in_direction, key_to_direction,
        legend::{LegendItem, LegendSection},
        render_helpers::{render_error, render_no_goals},
        Direction, FilterToggle, HypothesisFilters, InteractiveComponent, KeyMouseEvent, Selection,
        Theme,
    },
};

/// Input for updating the Deduction Tree mode.
pub struct DeductionTreeModeInput {
    pub state: TacticProofState,
    pub definition: Option<DefinitionInfo>,
    pub error: Option<String>,
    pub proof_dag: Option<CompleteProofDag>,
}

impl DeductionTreeModeInput {
    /// Create a builder for this input type.
    pub fn builder() -> DeductionTreeModeInputBuilder {
        DeductionTreeModeInputBuilder::default()
    }
}

/// Builder for `DeductionTreeModeInput`.
#[derive(Default)]
pub struct DeductionTreeModeInputBuilder {
    dag: Option<CompleteProofDag>,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
}

impl DeductionTreeModeInputBuilder {
    pub fn dag(mut self, dag: &CompleteProofDag) -> Self {
        self.dag = Some(dag.clone());
        self
    }

    pub fn definition(mut self, definition: Option<DefinitionInfo>) -> Self {
        self.definition = definition;
        self
    }

    pub fn error(mut self, error: Option<String>) -> Self {
        self.error = error;
        self
    }

    pub fn build(self) -> DeductionTreeModeInput {
        let state = self
            .dag
            .as_ref()
            .and_then(|d| d.current_node_id)
            .and_then(|id| self.dag.as_ref()?.get(id))
            .map(|node| node.proof_state_after.clone())
            .unwrap_or_default();

        DeductionTreeModeInput {
            state,
            definition: self.definition,
            error: self.error,
            proof_dag: self.dag,
        }
    }
}

/// Deduction Tree display mode - semantic tree visualization.
pub struct SemanticTableau {
    state: TacticProofState,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
    proof_dag: Option<CompleteProofDag>,
    filters: HypothesisFilters,
    /// Flat index into `tree_selectable_items()`.
    selected_idx: Option<usize>,
    /// When true, tree renders top-down (parent above children).
    tree_top_down: bool,
    /// State for the semantic tableau widget.
    tableau_state: SemanticTableauState,
}

impl Default for SemanticTableau {
    fn default() -> Self {
        Self {
            state: TacticProofState::default(),
            definition: None,
            error: None,
            proof_dag: None,
            filters: HypothesisFilters::default(),
            selected_idx: None,
            tree_top_down: true,
            tableau_state: SemanticTableauState::default(),
        }
    }
}

impl SemanticTableau {
    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    /// Build list of all selectable items in the proof tree.
    fn tree_selectable_items(&self) -> Vec<Selection> {
        let Some(dag) = &self.proof_dag else {
            return Vec::new();
        };
        if dag.is_empty() {
            return Vec::new();
        }

        let initial_hyps = dag
            .initial_proof_state()
            .hypotheses
            .iter()
            .enumerate()
            .filter(|(_, h)| !h.is_proof_term)
            .map(|(idx, _)| Selection::InitialHyp { hyp_idx: idx });

        let node_selections = dag.dfs_iter().flat_map(|node| {
            let node_id = node.id;
            let hyp_selections =
                node.new_hypothesis_indices
                    .iter()
                    .map(move |&hyp_idx| Selection::Hyp {
                        node_id,
                        hyp_idx: hyp_idx as usize,
                    });
            let goal_selections = (0..node.proof_state_after.goals.len())
                .map(move |goal_idx| Selection::Goal { node_id, goal_idx });
            hyp_selections.chain(goal_selections)
        });

        initial_hyps
            .chain(node_selections)
            .chain(std::iter::once(Selection::Theorem))
            .collect()
    }

    fn current_tree_selection(&self) -> Option<Selection> {
        let items = self.tree_selectable_items();
        self.selected_idx.and_then(|idx| items.get(idx).copied())
    }

    fn select_by_selection(&mut self, sel: Selection) {
        let items = self.tree_selectable_items();
        if let Some(idx) = items.iter().position(|s| *s == sel) {
            self.selected_idx = Some(idx);
        }
    }

    /// Get the active goal selection (first goal of the current node).
    fn active_goal_selection(&self) -> Option<Selection> {
        let dag = self.proof_dag.as_ref()?;
        let current_node_id = dag.current_node_id?;
        Some(Selection::Goal {
            node_id: current_node_id,
            goal_idx: 0,
        })
    }

    fn move_in_direction(&mut self, direction: Direction) -> bool {
        if self.selected_idx.is_none() {
            trace!("No selection, starting at active goal");
            if let Some(sel) = self.active_goal_selection() {
                self.select_by_selection(sel);
                return true;
            }
        }

        let current = self.current_tree_selection();
        let Some(current_sel) = current else {
            trace!("No current selection found");
            return false;
        };

        let items = self.tree_selectable_items();

        let navigation_regions = self.tableau_state.navigation_regions();

        trace!(
            ?current_sel,
            ?direction,
            nav_regions = navigation_regions.len(),
            selectable_items = items.len(),
            "move_in_direction called"
        );

        let result = find_nearest_in_direction(&navigation_regions, current_sel, direction);

        trace!(?result, "Navigation result in move_in_direction");

        result.is_some_and(|sel| {
            let found_in_items = items.contains(&sel);
            trace!(?sel, found_in_items, "Attempting to select");
            self.select_by_selection(sel);
            true
        })
    }
}

impl InteractiveComponent for SemanticTableau {
    type Input = DeductionTreeModeInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        let old_proof_count = self.proof_dag.as_ref().map_or(0, CompleteProofDag::len);
        let new_proof_count = input.proof_dag.as_ref().map_or(0, CompleteProofDag::len);

        let old_current = self.proof_dag.as_ref().and_then(|dag| dag.current_node_id);
        let new_current = input.proof_dag.as_ref().and_then(|dag| dag.current_node_id);

        let tree_changed = old_proof_count != new_proof_count || old_current != new_current;
        let state_changed = self.state.goals.len() != input.state.goals.len()
            || self.state.hypotheses.len() != input.state.hypotheses.len();

        self.tableau_state
            .update_current_node(input.proof_dag.as_ref().and_then(|dag| dag.current_node_id));

        self.state = input.state;
        self.definition = input.definition;
        self.error = input.error;
        self.proof_dag = input.proof_dag;

        if tree_changed || state_changed {
            if let Some(sel) = self.active_goal_selection() {
                self.select_by_selection(sel);
            } else {
                self.selected_idx = None;
            }
        }
    }

    fn handle_event(&mut self, event: Self::Event) -> bool {
        match event {
            KeyMouseEvent::Key(key) => {
                if let Some(dir) = key_to_direction(key.code) {
                    return self.move_in_direction(dir);
                }
                match key.code {
                    KeyCode::Char('t') => {
                        self.tree_top_down = !self.tree_top_down;
                        true
                    }
                    _ => false,
                }
            }
            KeyMouseEvent::Mouse(mouse) => {
                if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
                    return false;
                }

                if let Some(sel) = self.tableau_state.find_click_at(mouse.column, mouse.row) {
                    self.select_by_selection(sel);
                    return true;
                }
                false
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let content_area = render_error(frame, area, self.error.as_deref());

        if self.state.goals.is_empty() && self.proof_dag.is_none() {
            render_no_goals(frame, content_area);
            return;
        }

        if let Some(ref dag) = self.proof_dag {
            let widget = SemanticTableauLayout::new(
                dag,
                self.tree_top_down,
                self.current_tree_selection(),
                &self.state,
            );
            frame.render_stateful_widget(widget, content_area, &mut self.tableau_state);
        } else {
            frame.render_widget(
                Paragraph::new("No proof steps available").style(Style::new().fg(Color::DarkGray)),
                content_area,
            );
        }
    }
}

impl Mode for SemanticTableau {
    type Model = DeductionTreeModeInput;

    const NAME: &'static str = "Semantic tableau";
    const KEYBINDINGS: &'static [(&'static str, &'static str)] = &[("hjkl", "nav")];
    const SUPPORTED_FILTERS: &'static [FilterToggle] = &[];

    fn current_selection(&self) -> Option<Selection> {
        self.current_tree_selection()
    }

    fn legend_sections(&self) -> Vec<LegendSection> {
        vec![LegendSection::new(
            "Node States",
            Color::White,
            vec![
                LegendItem::new("■", Theme::CURRENT_NODE_BORDER, "current"),
                LegendItem::new("■", Theme::HYP_NAME, "hypothesis"),
                LegendItem::new("■", Theme::GOAL_FG, "goal"),
                LegendItem::new("■", Theme::COMPLETED_GOAL_FG, "completed"),
                LegendItem::new("■", Theme::INCOMPLETE_NODE_BORDER, "incomplete"),
            ],
        )]
    }
}
