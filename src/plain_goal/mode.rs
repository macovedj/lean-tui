use crossterm::event::{MouseButton, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::{
    deduction_tree::{types::CompleteProofDag, TacticProofState},
    display_mode::Mode,
    protocol::{DiagnosticInfo, NodeId},
    shared::{
        errors_section::{ErrorsSection, ErrorsSectionState},
        render_helpers::{render_error, render_no_goals},
        AssumptionPane, AssumptionPaneState, FilterToggle, GoalsPane, GoalsPaneState,
        HypothesisFilters, InteractiveComponent, KeyMouseEvent, Selection,
    },
};

/// Input for updating the Plain List mode.
pub struct PlainListInput {
    pub state: TacticProofState,
    pub error: Option<String>,
    pub proof_dag: Option<CompleteProofDag>,
}

impl PlainListInput {
    pub fn builder() -> PlainListInputBuilder {
        PlainListInputBuilder::default()
    }
}

#[derive(Default)]
pub struct PlainListInputBuilder {
    dag: Option<CompleteProofDag>,
    error: Option<String>,
}

impl PlainListInputBuilder {
    pub fn dag(mut self, dag: &CompleteProofDag) -> Self {
        self.dag = Some(dag.clone());
        self
    }

    pub fn error(mut self, error: Option<String>) -> Self {
        self.error = error;
        self
    }

    pub fn build(self) -> PlainListInput {
        let state = self
            .dag
            .as_ref()
            .and_then(|d| d.current_node_id)
            .and_then(|id| self.dag.as_ref()?.get(id))
            .map(|node| node.proof_state.clone())
            .unwrap_or_default();

        PlainListInput {
            state,
            error: self.error,
            proof_dag: self.dag,
        }
    }
}

/// Plain list mode - shows assumptions and goals panes.
#[derive(Default)]
pub struct PlainList {
    state: TacticProofState,
    current_node_id: Option<NodeId>,
    active_goal_name: Option<String>,
    error: Option<String>,
    filters: HypothesisFilters,
    current_node_errors: Vec<DiagnosticInfo>,
    errors_section_state: ErrorsSectionState,
    assumption_state: AssumptionPaneState,
    goals_state: GoalsPaneState,
}

impl PlainList {
    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    pub fn take_clicked_error(&mut self) -> Option<String> {
        self.errors_section_state.take_clicked_error()
    }
}

impl InteractiveComponent for PlainList {
    type Input = PlainListInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        let current_node_id = input.proof_dag.as_ref().and_then(|dag| dag.current_node_id);
        let current_node = current_node_id.and_then(|id| input.proof_dag.as_ref()?.get(id));

        self.current_node_id = current_node_id;
        self.active_goal_name = current_node
            .and_then(|node| node.proof_state.goals.first())
            .and_then(|g| g.username_str().map(String::from));
        self.current_node_errors = current_node
            .map(|node| node.diagnostics.clone())
            .unwrap_or_default();
        self.state = input.state;
        self.error = input.error;
    }

    fn handle_event(&mut self, event: Self::Event) -> bool {
        match event {
            KeyMouseEvent::Key(_) => false,
            KeyMouseEvent::Mouse(mouse) => {
                if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
                    return false;
                }
                self.errors_section_state
                    .handle_click(mouse.column, mouse.row)
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let content_area = render_error(frame, area, self.error.as_deref());

        if self.state.goals.is_empty() && self.state.hypotheses.is_empty() {
            render_no_goals(frame, content_area);
            return;
        }

        let error_lines: usize = self
            .current_node_errors
            .iter()
            .map(|e| e.message.lines().count())
            .sum();

        let (main_area, errors_area) = if error_lines > 0 {
            let max_error_height = (content_area.height / 3).max(3);
            let error_height = ((error_lines as u16) + 2).min(max_error_height);
            let [main, errors] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(error_height)])
                    .areas(content_area);
            (main, Some(errors))
        } else {
            (content_area, None)
        };

        // Two-row layout: assumptions on top, goals on bottom
        let [hyp_area, goal_area] =
            Layout::vertical([Constraint::Percentage(40), Constraint::Percentage(60)])
                .areas(main_area);

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

        if let Some(errors_area) = errors_area {
            self.errors_section_state
                .update(self.current_node_errors.clone());
            frame.render_stateful_widget(
                ErrorsSection,
                errors_area,
                &mut self.errors_section_state,
            );
        }
    }
}

impl Mode for PlainList {
    type Model = PlainListInput;

    const NAME: &'static str = "Plain";
    const KEYBINDINGS: &'static [(&'static str, &'static str)] = &[];
    const SUPPORTED_FILTERS: &'static [FilterToggle] = &[];

    fn current_selection(&self) -> Option<Selection> {
        None
    }
}
