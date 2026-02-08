use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Color,
    Frame,
};

use crate::{
    deduction_tree::{
        types::{CompleteProofDag, ProofDagTacticNode},
        TacticProofState,
    },
    display_mode::Mode,
    protocol::NodeId,
    shared::{
        errors_section::{ErrorsSection, ErrorsSectionState},
        legend::{LegendItem, LegendSection},
        render_helpers::{render_error, render_no_goals},
        theme::Theme,
        AssumptionPane, AssumptionPaneState, FilterToggle, GoalsPane, GoalsPaneState,
        HypothesisFilters, InteractiveComponent, InteractiveStatefulWidget, KeyMouseEvent,
        Selection,
    },
    tactic_tree::proof_steps_sidebar::{ProofStepsSidebar, ProofStepsSidebarState},
};

/// Input for updating the Steps mode.
pub struct StepsModeInput {
    pub state: TacticProofState,
    pub error: Option<String>,
    pub proof_dag: Option<CompleteProofDag>,
}

impl StepsModeInput {
    /// Create a builder for this input type.
    pub fn builder() -> StepsModeInputBuilder {
        StepsModeInputBuilder::default()
    }
}

/// Builder for `StepsModeInput`.
#[derive(Default)]
pub struct StepsModeInputBuilder {
    dag: Option<CompleteProofDag>,
    error: Option<String>,
}

impl StepsModeInputBuilder {
    pub fn dag(mut self, dag: &CompleteProofDag) -> Self {
        self.dag = Some(dag.clone());
        self
    }

    pub fn error(mut self, error: Option<String>) -> Self {
        self.error = error;
        self
    }

    pub fn build(self) -> StepsModeInput {
        let state = self
            .dag
            .as_ref()
            .and_then(|d| d.current_node_id)
            .and_then(|id| self.dag.as_ref()?.get(id))
            .map(|node| node.proof_state.clone())
            .unwrap_or_default();

        StepsModeInput {
            state,
            error: self.error,
            proof_dag: self.dag,
        }
    }
}

/// Tactic tree display mode - shows the proof steps sidebar with
/// assumptions/goals in right column.
#[derive(Default)]
pub struct TacticTree {
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
    sidebar_viewport_height: usize,
}

impl TacticTree {
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

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        self.handle_sidebar_navigation(key.code)
    }

    fn handle_sidebar_navigation(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.sidebar_state.scroll_down(self.sidebar_viewport_height);
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.sidebar_state.scroll_up();
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

        self.errors_section_state
            .handle_click(mouse.column, mouse.row)
    }

    pub fn take_clicked_error(&mut self) -> Option<String> {
        self.errors_section_state.take_clicked_error()
    }
}

impl InteractiveComponent for TacticTree {
    type Input = StepsModeInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        self.state = input.state;
        self.error = input.error;
        self.current_node_id = input.proof_dag.as_ref().and_then(|dag| dag.current_node_id);
        self.active_goal_name = self
            .current_node_id
            .and_then(|id| input.proof_dag.as_ref()?.get(id))
            .and_then(|node| node.proof_state.goals.first())
            .and_then(|g| g.username_str().map(String::from));
        self.proof_dag = input.proof_dag;
    }

    fn handle_event(&mut self, event: Self::Event) -> bool {
        match event {
            KeyMouseEvent::Key(key) => self.handle_key(key),
            KeyMouseEvent::Mouse(mouse) => self.handle_mouse(mouse),
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
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

        // Two-column layout: sidebar (60%) | assumptions+goals (40%)
        let [sidebar_area, right_area] =
            Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)])
                .areas(main_area);

        // Render sidebar
        ProofStepsSidebar::update_state(&mut self.sidebar_state, self.proof_dag.clone());
        self.sidebar_state.set_focused(true);
        self.sidebar_viewport_height = sidebar_area.height.saturating_sub(2) as usize;
        frame.render_stateful_widget(ProofStepsSidebar, sidebar_area, &mut self.sidebar_state);

        // Right column: assumptions on top (40%), goals on bottom (60%)
        let [hyp_area, goal_area] =
            Layout::vertical([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(right_area);

        let is_current = self.current_node_id.is_some();

        frame.render_stateful_widget(
            AssumptionPane::new(
                &self.state,
                self.filters,
                None,
                false,
                is_current,
                self.current_node_id,
            ),
            hyp_area,
            &mut self.assumption_state,
        );

        frame.render_stateful_widget(
            GoalsPane::new(
                &self.state,
                None,
                false,
                is_current,
                self.current_node_id,
                self.active_goal_name.as_deref(),
            ),
            goal_area,
            &mut self.goals_state,
        );

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

impl Mode for TacticTree {
    type Model = StepsModeInput;
    const NAME: &'static str = "Tactic tree";
    const KEYBINDINGS: &'static [(&'static str, &'static str)] = &[("hjkl", "scroll")];
    const SUPPORTED_FILTERS: &'static [FilterToggle] = &[];

    fn current_selection(&self) -> Option<Selection> {
        None
    }

    fn legend_sections(&self) -> Vec<LegendSection> {
        vec![LegendSection::new(
            "Tactic Tree",
            Color::Yellow,
            vec![
                LegendItem::new("▶", Theme::CURRENT_NODE_BORDER, "current step"),
                LegendItem::new("⊢", Theme::GOAL, "open goal"),
            ],
        )]
    }
}
