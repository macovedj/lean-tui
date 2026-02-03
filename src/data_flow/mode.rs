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
    data_flow::layout::{FunctionalTableauLayout, FunctionalTableauState},
    display_mode::Mode,
    protocol::GenericDag,
    shared::{
        find_nearest_in_direction,
        legend::{LegendItem, LegendSection},
        render_helpers::render_error,
        Direction, FilterToggle, HypothesisFilters, InteractiveComponent, KeyMouseEvent, Selection,
        Theme,
    },
};

/// Input for updating the Data Flow mode.
pub struct DataFlowModeInput {
    pub definition: Option<DefinitionInfo>,
    pub error: Option<String>,
    pub generic_dag: Option<GenericDag>,
}

/// Data Flow display mode - functional DAG visualization.
pub struct DataFlowMode {
    definition: Option<DefinitionInfo>,
    error: Option<String>,
    generic_dag: Option<GenericDag>,
    filters: HypothesisFilters,
    /// Flat index into `tree_selectable_items()`.
    selected_idx: Option<usize>,
    /// When true, tree renders top-down (parent above children).
    tree_top_down: bool,
    /// State for the functional tableau widget.
    functional_tableau_state: FunctionalTableauState,
}

impl Default for DataFlowMode {
    fn default() -> Self {
        Self {
            definition: None,
            error: None,
            generic_dag: None,
            filters: HypothesisFilters::default(),
            selected_idx: None,
            tree_top_down: true,
            functional_tableau_state: FunctionalTableauState::default(),
        }
    }
}

impl DataFlowMode {
    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    /// Build selectable items for generic DAG.
    fn tree_selectable_items(&self) -> Vec<Selection> {
        let Some(dag) = &self.generic_dag else {
            return Vec::new();
        };

        if dag.is_empty() {
            return Vec::new();
        }

        let mut items = Vec::new();

        // Initial bindings (function parameters)
        for (idx, b) in dag.initial_context.iter().enumerate() {
            if !b.is_implicit && !b.is_instance {
                items.push(Selection::InitialHyp { hyp_idx: idx });
            }
        }

        // For each node: add as goal-like selection
        for node in dag.dfs_iter() {
            let node_id = node.id;

            // Each node is selectable
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
        let dag = self.generic_dag.as_ref()?;
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
        let navigation_regions = self.functional_tableau_state.navigation_regions();

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

impl InteractiveComponent for DataFlowMode {
    type Input = DataFlowModeInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        // Detect changes to generic DAG
        let old_count = self.generic_dag.as_ref().map_or(0, GenericDag::len);
        let new_count = input.generic_dag.as_ref().map_or(0, GenericDag::len);

        let old_current = self
            .generic_dag
            .as_ref()
            .and_then(|dag| dag.current_node_id);
        let new_current = input
            .generic_dag
            .as_ref()
            .and_then(|dag| dag.current_node_id);

        let tree_changed = old_count != new_count || old_current != new_current;

        // Update state when current node changes
        self.functional_tableau_state.update_current_node(
            input
                .generic_dag
                .as_ref()
                .and_then(|dag| dag.current_node_id),
        );

        self.definition = input.definition;
        self.error = input.error;
        self.generic_dag = input.generic_dag;

        // Auto-select active goal when tree changes
        if tree_changed {
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
                KeyCode::Char('s') => {
                    self.functional_tableau_state.toggle_stretch();
                    true
                }
                _ => false,
            },
            KeyMouseEvent::Mouse(mouse) => {
                if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
                    return false;
                }

                let clicked = self
                    .functional_tableau_state
                    .find_click_at(mouse.column, mouse.row);

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

        // Render generic DAG if present
        if let Some(ref dag) = self.generic_dag {
            let widget = FunctionalTableauLayout::new(
                dag,
                self.tree_top_down,
                self.functional_tableau_state.stretch_enabled(),
                self.current_tree_selection(),
            );
            frame.render_stateful_widget(widget, content_area, &mut self.functional_tableau_state);
            return;
        }

        // No DAG available
        frame.render_widget(
            Paragraph::new("No data flow available (term mode only)")
                .style(Style::new().fg(Color::DarkGray)),
            content_area,
        );
    }
}

impl Mode for DataFlowMode {
    type Model = DataFlowModeInput;

    const NAME: &'static str = "Data Flow";
    const KEYBINDINGS: &'static [(&'static str, &'static str)] =
        &[("hjkl", "nav"), ("t", "flip"), ("s", "stretch")];
    const SUPPORTED_FILTERS: &'static [FilterToggle] = &[];

    fn current_selection(&self) -> Option<Selection> {
        self.current_tree_selection()
    }

    fn legend_sections(&self) -> Vec<LegendSection> {
        vec![LegendSection::new(
            "Node States",
            Color::White,
            vec![
                LegendItem::new("■", Theme::HYP_NAME, "binding"),
                LegendItem::new("■", Theme::GOAL_FG, "expression"),
                LegendItem::new("■", Theme::COMPLETED_GOAL_FG, "complete"),
                LegendItem::new("■", Theme::INCOMPLETE_NODE_BORDER, "incomplete"),
            ],
        )]
    }
}
