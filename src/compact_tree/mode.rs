use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Color,
    Frame,
};

use crate::{
    compact_tree::proof_steps_sidebar::{ProofStepsSidebar, ProofStepsSidebarState},
    deduction_tree::{
        types::{CompleteProofDag, ProofDagTacticNode},
        TacticProofState,
    },
    display_mode::Mode,
    protocol::NodeId,
    shared::{
        errors_section::{ErrorsSection, ErrorsSectionState},
        hypothesis_indices,
        legend::{LegendItem, LegendSection},
        render_helpers::{render_error, render_no_goals},
        theme::Theme,
        AssumptionPane, AssumptionPaneState, FilterToggle, GoalsPane, GoalsPaneState,
        HypothesisFilters, InteractiveComponent, InteractiveStatefulWidget, KeyMouseEvent,
        ListSelection, Selection,
    },
};

/// Input for updating the Compact Tree mode.
pub struct CompactTreeInput {
    pub dag: Option<CompleteProofDag>,
    pub error: Option<String>,
}

/// Compact tree display mode - shows the proof steps sidebar with
/// assumptions/goals in right column.
#[derive(Default)]
pub struct CompactTree {
    state: TacticProofState,
    error: Option<String>,
    proof_dag: Option<CompleteProofDag>,
    current_node_id: Option<NodeId>,
    active_goal_name: Option<String>,
    errors_section_state: ErrorsSectionState,
    sidebar_state: ProofStepsSidebarState,
    assumption_state: AssumptionPaneState,
    goals_state: GoalsPaneState,
    filters: HypothesisFilters,
    selection: ListSelection,
}

impl CompactTree {
    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    fn has_steps(&self) -> bool {
        self.proof_dag.as_ref().is_some_and(|dag| !dag.is_empty())
    }

    fn current_node(&self) -> Option<&ProofDagTacticNode> {
        self.proof_dag
            .as_ref()
            .and_then(|dag| dag.current_node_id.and_then(|id| dag.get(id)))
    }

    fn selectable_items(&self) -> Vec<Selection> {
        let Some(node_id) = self.current_node_id else {
            return Vec::new();
        };

        let hyp_items = hypothesis_indices(
            self.state.hypotheses.len(),
            self.filters.reverse_order,
        )
        .filter(|&hyp_idx| {
            self.state.hypotheses.get(hyp_idx).is_some_and(|h| {
                (!self.filters.hide_instances || !h.is_typeclass_instance)
                    && (!self.filters.hide_inaccessible || !h.is_proof_term)
            })
        })
        .map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });

        let goal_items = (0..self.state.goals.len())
            .map(move |goal_idx| Selection::Goal { node_id, goal_idx });

        hyp_items.chain(goal_items).collect()
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        let items = self.selectable_items();
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.selection.select_next(items.len());
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selection.select_previous(items.len());
                true
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.sidebar_state.scroll_left();
                true
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.sidebar_state.scroll_right();
                true
            }
            _ => false,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> bool {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }

        if self
            .errors_section_state
            .handle_click(mouse.column, mouse.row)
        {
            return true;
        }

        let items = self.selectable_items();
        self.selection.handle_click(mouse.column, mouse.row, &items)
    }

    pub fn take_clicked_error(&mut self) -> Option<String> {
        self.errors_section_state.take_clicked_error()
    }
}

impl InteractiveComponent for CompactTree {
    type Input = CompactTreeInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        self.current_node_id = input.dag.as_ref().and_then(|dag| dag.current_node_id);
        let current_node = self
            .current_node_id
            .and_then(|id| input.dag.as_ref()?.get(id));

        let new_state = current_node
            .map(|node| node.proof_state.clone())
            .unwrap_or_default();

        let state_changed = self.state.goals.len() != new_state.goals.len()
            || self.state.hypotheses.len() != new_state.hypotheses.len();

        self.state = new_state;
        self.active_goal_name = current_node
            .and_then(|node| node.proof_state.goals.first())
            .and_then(|g| g.username_str().map(String::from));
        self.error = input.error;
        self.proof_dag = input.dag;

        if state_changed {
            self.selection.reset(self.selectable_items().len());
        }
    }

    fn handle_event(&mut self, event: Self::Event) -> bool {
        match event {
            KeyMouseEvent::Key(key) => self.handle_key(key),
            KeyMouseEvent::Mouse(mouse) => self.handle_mouse(mouse),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.selection.clear_regions();

        let content = render_error(frame, area, self.error.as_deref());

        if self.state.goals.is_empty() && !self.has_steps() {
            render_no_goals(frame, content);
            return;
        }

        // Get errors from current node
        let current_node_errors = self
            .current_node()
            .map(|node| node.diagnostics.clone())
            .unwrap_or_default();

        let error_lines: usize = current_node_errors
            .iter()
            .map(|e| e.message.lines().count())
            .sum();

        // Layout: main area with errors at bottom if present
        let (main_area, errors_area) = if error_lines > 0 {
            let max_error_height = (content.height / 3).max(3);
            let error_height = ((error_lines as u16) + 2).min(max_error_height);
            let [main, errors] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(error_height)])
                    .areas(content);
            (main, Some(errors))
        } else {
            (content, None)
        };

        // Update sidebar state first to calculate required width
        ProofStepsSidebar::update_state(&mut self.sidebar_state, self.proof_dag.clone());

        // Dynamic sidebar width: fit content with min 20 and max 60% of available width
        let required_width = self.sidebar_state.required_width();
        let max_sidebar = (main_area.width * 60) / 100;
        let sidebar_width = required_width.clamp(20, max_sidebar);

        // Two-column layout: sidebar (fit content) | assumptions+goals (fill remaining)
        let [sidebar_area, right_area] =
            Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Fill(1)])
                .areas(main_area);
        frame.render_stateful_widget(ProofStepsSidebar, sidebar_area, &mut self.sidebar_state);

        // Right column: assumptions on top (40%), goals on bottom (60%)
        let [hyp_area, goal_area] =
            Layout::vertical([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(right_area);

        let is_current = self.current_node_id.is_some();
        let selection = self.current_selection();

        frame.render_stateful_widget(
            AssumptionPane::new(
                &self.state,
                self.filters,
                selection,
                true,
                is_current,
                self.current_node_id,
            ),
            hyp_area,
            &mut self.assumption_state,
        );

        for region in self.assumption_state.click_regions() {
            self.selection.add_region(
                Rect::new(
                    region.x as u16,
                    region.y as u16,
                    region.width,
                    region.height,
                ),
                region.selection,
            );
        }

        frame.render_stateful_widget(
            GoalsPane::new(
                &self.state,
                selection,
                true,
                is_current,
                self.current_node_id,
                self.active_goal_name.as_deref(),
            ),
            goal_area,
            &mut self.goals_state,
        );

        for region in self.goals_state.click_regions() {
            self.selection.add_region(
                Rect::new(
                    region.x as u16,
                    region.y as u16,
                    region.width,
                    region.height,
                ),
                region.selection,
            );
        }

        // Render errors
        if let Some(errors_area) = errors_area {
            self.errors_section_state.update(current_node_errors);
            frame.render_stateful_widget(
                ErrorsSection,
                errors_area,
                &mut self.errors_section_state,
            );
        }
    }
}

impl Mode for CompactTree {
    type Model = CompactTreeInput;
    const NAME: &'static str = "Compact Tree";
    const KEYBINDINGS: &'static [(&'static str, &'static str)] = &[("jk", "select"), ("hl", "scroll")];
    const SUPPORTED_FILTERS: &'static [FilterToggle] = &[];

    fn current_selection(&self) -> Option<Selection> {
        let items = self.selectable_items();
        self.selection.current_selection(&items).copied()
    }

    fn legend_sections(&self) -> Vec<LegendSection> {
        vec![LegendSection::new(
            "Compact Tree",
            Color::Yellow,
            vec![
                LegendItem::new("▶", Theme::CURRENT_NODE_BORDER, "current step"),
                LegendItem::new("⊢", Theme::GOAL, "open goal"),
            ],
        )]
    }
}
