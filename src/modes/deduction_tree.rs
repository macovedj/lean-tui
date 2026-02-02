//! Deduction Tree mode - semantic tree visualization.

use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};
use tracing::trace;

use super::Mode;
use crate::{
    app::DefinitionInfo,
    protocol::{CompleteFunctionalDag, CompleteProofDag, TacticProofState},
    widgets::{
        render_helpers::{render_error, render_no_goals},
        tableau::{
            find_nearest_in_direction, Direction, FunctionalTableauLayout, FunctionalTableauState,
            SemanticTableauLayout, SemanticTableauState,
        },
        FilterToggle, HypothesisFilters, InteractiveComponent, KeyMouseEvent, Selection,
    },
};

/// Input for updating the Deduction Tree mode.
pub struct DeductionTreeModeInput {
    pub state: TacticProofState,
    pub definition: Option<DefinitionInfo>,
    pub error: Option<String>,
    pub proof_dag: Option<CompleteProofDag>,
    pub functional_dag: Option<CompleteFunctionalDag>,
}

/// Deduction Tree display mode - semantic tree visualization.
pub struct SemanticTableau {
    state: TacticProofState,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
    proof_dag: Option<CompleteProofDag>,
    functional_dag: Option<CompleteFunctionalDag>,
    filters: HypothesisFilters,
    /// Flat index into `tree_selectable_items()`.
    selected_idx: Option<usize>,
    /// When true, tree renders top-down (parent above children).
    tree_top_down: bool,
    /// State for the semantic tableau widget.
    tableau_state: SemanticTableauState,
    /// State for the functional tableau widget.
    functional_tableau_state: FunctionalTableauState,
}

impl Default for SemanticTableau {
    fn default() -> Self {
        Self {
            state: TacticProofState::default(),
            definition: None,
            error: None,
            proof_dag: None,
            functional_dag: None,
            filters: HypothesisFilters::default(),
            selected_idx: None,
            tree_top_down: true,
            tableau_state: SemanticTableauState::default(),
            functional_tableau_state: FunctionalTableauState::default(),
        }
    }
}

impl SemanticTableau {
    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    /// Check if we're displaying a functional DAG.
    fn is_functional_mode(&self) -> bool {
        self.functional_dag.is_some()
    }

    /// Build list of all selectable items in the tree from `proof_dag` or
    /// `functional_dag`.
    fn tree_selectable_items(&self) -> Vec<Selection> {
        if let Some(dag) = &self.functional_dag {
            return Self::functional_selectable_items(dag);
        }

        let Some(dag) = &self.proof_dag else {
            return Vec::new();
        };
        if dag.is_empty() {
            return Vec::new();
        }

        let mut items = Vec::new();

        // Initial hypotheses from the initial_proof_state
        let initial = dag.initial_proof_state();
        for (idx, h) in initial.hypotheses.iter().enumerate() {
            if !h.is_proof_term {
                items.push(Selection::InitialHyp { hyp_idx: idx });
            }
        }

        // For each node: new hypotheses and goals
        for node in dag.dfs_iter() {
            let node_id = node.id;

            // New hypotheses introduced by this step
            for &hyp_idx in &node.new_hypothesis_indices {
                items.push(Selection::Hyp {
                    node_id,
                    hyp_idx: hyp_idx as usize,
                });
            }

            // Goals after this step
            for (goal_idx, _) in node.proof_state_after.goals.iter().enumerate() {
                items.push(Selection::Goal { node_id, goal_idx });
            }
        }

        // Theorem (final goal)
        items.push(Selection::Theorem);

        items
    }

    /// Build selectable items for functional DAG.
    fn functional_selectable_items(dag: &CompleteFunctionalDag) -> Vec<Selection> {
        if dag.is_empty() {
            return Vec::new();
        }

        let mut items = Vec::new();

        // Initial bindings (function parameters)
        for (idx, b) in dag.initial_bindings.iter().enumerate() {
            if !b.is_implicit && !b.is_instance {
                items.push(Selection::InitialHyp { hyp_idx: idx });
            }
        }

        // For each node: new bindings and expected type
        for node in dag.dfs_iter() {
            let node_id = node.id;

            // New bindings introduced by this step
            for &binding_idx in &node.new_binding_indices {
                items.push(Selection::Hyp {
                    node_id,
                    hyp_idx: binding_idx as usize,
                });
            }

            // Expected type (like a goal)
            items.push(Selection::Goal {
                node_id,
                goal_idx: 0,
            });
        }

        // Definition type (like theorem)
        items.push(Selection::Theorem);

        items
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
        if let Some(dag) = &self.functional_dag {
            let current_node_id = dag.current_node_id?;
            return Some(Selection::Goal {
                node_id: current_node_id,
                goal_idx: 0,
            });
        }

        let dag = self.proof_dag.as_ref()?;
        let current_node_id = dag.current_node_id?;
        Some(Selection::Goal {
            node_id: current_node_id,
            goal_idx: 0,
        })
    }

    fn move_in_direction(&mut self, direction: Direction) -> bool {
        // If nothing is selected, start at the active goal
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

        // Use the appropriate navigation based on which DAG is active
        let navigation_regions = if self.is_functional_mode() {
            self.functional_tableau_state.navigation_regions()
        } else {
            self.tableau_state.proof.navigation_regions()
        };

        trace!(
            ?current_sel,
            ?direction,
            nav_regions = navigation_regions.len(),
            selectable_items = items.len(),
            "move_in_direction called"
        );

        let result = find_nearest_in_direction(navigation_regions, current_sel, direction);

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
        // Detect changes to proof tree or goals
        let old_proof_count = self.proof_dag.as_ref().map_or(0, CompleteProofDag::len);
        let new_proof_count = input.proof_dag.as_ref().map_or(0, CompleteProofDag::len);
        let old_functional_count = self
            .functional_dag
            .as_ref()
            .map_or(0, CompleteFunctionalDag::len);
        let new_functional_count = input
            .functional_dag
            .as_ref()
            .map_or(0, CompleteFunctionalDag::len);

        let old_current = self
            .proof_dag
            .as_ref()
            .and_then(|dag| dag.current_node_id)
            .or_else(|| {
                self.functional_dag
                    .as_ref()
                    .and_then(|dag| dag.current_node_id)
            });
        let new_current = input
            .proof_dag
            .as_ref()
            .and_then(|dag| dag.current_node_id)
            .or_else(|| {
                input
                    .functional_dag
                    .as_ref()
                    .and_then(|dag| dag.current_node_id)
            });

        let tree_changed = old_proof_count != new_proof_count
            || old_functional_count != new_functional_count
            || old_current != new_current;
        let state_changed = self.state.goals.len() != input.state.goals.len()
            || self.state.hypotheses.len() != input.state.hypotheses.len();

        // Update state when current node changes
        self.tableau_state
            .update_current_node(input.proof_dag.as_ref().and_then(|dag| dag.current_node_id));
        self.functional_tableau_state.update_current_node(
            input
                .functional_dag
                .as_ref()
                .and_then(|dag| dag.current_node_id),
        );

        self.state = input.state;
        self.definition = input.definition;
        self.error = input.error;
        self.proof_dag = input.proof_dag;
        self.functional_dag = input.functional_dag;

        // Auto-select active goal when tree or goals change
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
            KeyMouseEvent::Key(key) => match key.code {
                KeyCode::Char('j') | KeyCode::Down => self.move_in_direction(Direction::Down),
                KeyCode::Char('k') | KeyCode::Up => self.move_in_direction(Direction::Up),
                KeyCode::Char('h') | KeyCode::Left => self.move_in_direction(Direction::Left),
                KeyCode::Char('l') | KeyCode::Right => self.move_in_direction(Direction::Right),
                KeyCode::Char('t') => {
                    self.tree_top_down = !self.tree_top_down;
                    true
                }
                _ => false,
            },
            KeyMouseEvent::Mouse(mouse) => {
                if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
                    return false;
                }

                let clicked = if self.is_functional_mode() {
                    self.functional_tableau_state
                        .find_click_at(mouse.column, mouse.row)
                } else {
                    self.tableau_state.find_click_at(mouse.column, mouse.row)
                };

                if let Some(sel) = clicked {
                    self.select_by_selection(sel);
                    return true;
                }
                false
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let content_area = render_error(frame, area, self.error.as_deref());

        // Render functional DAG if present
        if let Some(ref dag) = self.functional_dag {
            let widget = FunctionalTableauLayout::new(
                dag,
                self.tree_top_down,
                self.current_tree_selection(),
            );
            frame.render_stateful_widget(widget, content_area, &mut self.functional_tableau_state);
            return;
        }

        // Render proof DAG if present
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
}
