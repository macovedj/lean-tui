pub mod layout;
mod proof_pane;
mod state_node;
pub mod types;

use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};
use tracing::trace;
pub use types::{
    CompleteProofDag, ProofContextHypothesis, ProofDagConversionError, TacticProofState,
};

use crate::{
    deduction_tree::layout::{SemanticTableauLayout, SemanticTableauState},
    display_mode::Mode,
    interactive::{InteractiveComponent, KeyMouseEvent},
    navigation::{find_nearest_in_direction, key_to_direction, Direction},
    protocol::DiagnosticSeverity,
    selection::Selection,
    theme::Theme,
    widgets::{
        legend::{LegendItem, LegendSection},
        render_helpers::{render_error, render_no_goals},
    },
};

/// Input for updating the Deduction Tree mode.
pub struct DeductionTreeModeInput {
    pub dag: Option<CompleteProofDag>,
    pub error: Option<String>,
}

/// Deduction Tree display mode - semantic tree visualization.
pub struct SemanticTableau {
    state: TacticProofState,
    error: Option<String>,
    proof_dag: Option<CompleteProofDag>,
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
            error: None,
            proof_dag: None,
            selected_idx: None,
            tree_top_down: true,
            tableau_state: SemanticTableauState::default(),
        }
    }
}

impl SemanticTableau {
    fn tree_selectable_items(&self) -> Vec<Selection> {
        let Some(dag) = &self.proof_dag else {
            return Vec::new();
        };
        if dag.is_empty() {
            return Vec::new();
        }

        // Input/output panes are now global - only include node-specific selections
        dag.dfs_iter()
            .flat_map(|node| {
                let node_id = node.id;
                let hyp_selections =
                    node.new_hypothesis_indices
                        .iter()
                        .map(move |&hyp_idx| Selection::Hyp {
                            node_id,
                            hyp_idx: hyp_idx as usize,
                        });
                let active_goal_count = node
                    .proof_state
                    .goals
                    .iter()
                    .filter(|g| !g.is_removed)
                    .count();
                let goal_selections = (0..active_goal_count)
                    .map(move |goal_idx| Selection::Goal { node_id, goal_idx });
                let error_count = node
                    .diagnostics
                    .iter()
                    .filter(|d| d.severity == DiagnosticSeverity::Error)
                    .count();
                let error_selections =
                    (0..error_count).map(move |error_idx| Selection::Error { node_id, error_idx });
                hyp_selections
                    .chain(goal_selections)
                    .chain(error_selections)
            })
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
        let new_proof_count = input.dag.as_ref().map_or(0, CompleteProofDag::len);

        let old_current = self.proof_dag.as_ref().and_then(|dag| dag.current_node_id);
        let new_current = input.dag.as_ref().and_then(|dag| dag.current_node_id);

        let new_state = new_current
            .and_then(|id| input.dag.as_ref()?.get(id))
            .map(|node| node.proof_state.clone())
            .unwrap_or_default();

        let tree_changed = old_proof_count != new_proof_count || old_current != new_current;
        let state_changed = self.state.goals.len() != new_state.goals.len()
            || self.state.hypotheses.len() != new_state.hypotheses.len();

        self.tableau_state.update_current_node(new_current);

        self.state = new_state;
        self.error = input.error;
        self.proof_dag = input.dag;

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

    fn name(&self) -> &'static str {
        "Semantic tableau"
    }

    fn keybindings(&self) -> &'static [(&'static str, &'static str)] {
        &[
            ("hjkl", "nav"),
            // TODO flip upside down with `m` (mirror), currently with t but broken
        ]
    }

    fn current_selection(&self) -> Option<Selection> {
        self.current_tree_selection()
    }

    fn legend_sections(&self) -> Vec<LegendSection> {
        vec![
            LegendSection::new(
                "Hypotheses and goals",
                Color::White,
                vec![
                    LegendItem::new("h", Theme::HYP_NAME, "hypothesis"),
                    LegendItem::new("P", Theme::GOAL, "goal and theorem statement"),
                ],
            ),
            LegendSection::new(
                "Proof steps",
                Color::White,
                vec![
                    LegendItem::new("-", Theme::CURRENT_NODE_BORDER, "cursor"),
                    LegendItem::new("-", Theme::INACTIVE_NODE_BORDER, "inactive"),
                    LegendItem::new("-", Theme::COMPLETED_GOAL_FG, "completed"),
                    LegendItem::new("-", Theme::INCOMPLETE_NODE_BORDER, "incomplete"),
                    LegendItem::new("-", Theme::EDGE_ERROR_PROPAGATION, "type error"),
                ],
            ),
        ]
    }
}
