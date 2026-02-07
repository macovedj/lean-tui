use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    Frame,
};

use crate::{
    app::DefinitionInfo,
    deduction_tree::{types::CompleteProofDag, TacticProofState},
    display_mode::Mode,
    plain_goal::widget::OpenGoalList,
    protocol::{DiagnosticInfo, NodeId},
    shared::{
        errors_section::{ErrorsSection, ErrorsSectionState},
        hypothesis_indices, render_helpers::render_error, FilterToggle, HypothesisFilters,
        InteractiveComponent, KeyMouseEvent, ListSelection, Selection,
    },
};

/// Input for updating the Open Goal List mode.
pub struct PlainListInput {
    pub state: TacticProofState,
    pub definition: Option<DefinitionInfo>,
    pub error: Option<String>,
    pub proof_dag: Option<CompleteProofDag>,
}

impl PlainListInput {
    /// Create a builder for this input type.
    pub fn builder() -> PlainListInputBuilder {
        PlainListInputBuilder::default()
    }
}

/// Builder for `PlainListInput`.
#[derive(Default)]
pub struct PlainListInputBuilder {
    dag: Option<CompleteProofDag>,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
}

impl PlainListInputBuilder {
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

    pub fn build(self) -> PlainListInput {
        let state = self
            .dag
            .as_ref()
            .and_then(|d| d.current_node_id)
            .and_then(|id| self.dag.as_ref()?.get(id))
            .map(|node| node.proof_state_after.clone())
            .unwrap_or_default();

        PlainListInput {
            state,
            definition: self.definition,
            error: self.error,
            proof_dag: self.dag,
        }
    }
}

/// Open Goal List display mode - navigable list of open goals with hypotheses.
#[derive(Default)]
pub struct PlainList {
    /// Current proof state.
    state: TacticProofState,
    /// Current node ID in the DAG (for building selections).
    current_node_id: Option<NodeId>,
    /// Name of the goal the cursor's tactic is working on.
    active_goal_name: Option<String>,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
    filters: HypothesisFilters,
    selection: ListSelection,
    /// Errors from the current node.
    current_node_errors: Vec<DiagnosticInfo>,
    errors_section_state: ErrorsSectionState,
}

impl PlainList {
    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    fn selectable_items(&self) -> Vec<Selection> {
        let Some(node_id) = self.current_node_id else {
            return Vec::new();
        };

        let hyp_count = self.state.hypotheses.len();
        let goal_count = self.state.goals.len();

        let hyp_items = hypothesis_indices(hyp_count, self.filters.reverse_order)
            .filter(|&i| self.should_show_hypothesis(i))
            .map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });

        let goal_items = (0..goal_count).map(move |goal_idx| Selection::Goal { node_id, goal_idx });

        hyp_items.chain(goal_items).collect()
    }

    fn should_show_hypothesis(&self, idx: usize) -> bool {
        let Some(h) = self.state.hypotheses.get(idx) else {
            return false;
        };
        if self.filters.hide_instances && h.is_typeclass_instance {
            return false;
        }
        if self.filters.hide_inaccessible && h.is_proof_term {
            return false;
        }
        true
    }

    /// Take the clicked error message (if any) and clear it.
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

        let state_changed = self.state.goals.len() != input.state.goals.len()
            || self.state.hypotheses.len() != input.state.hypotheses.len();

        self.current_node_id = current_node_id;
        self.active_goal_name = current_node
            .and_then(|node| node.proof_state_before.goals.first())
            .and_then(|g| g.username_str().map(String::from));
        self.current_node_errors = current_node
            .map(|node| node.diagnostics.clone())
            .unwrap_or_default();
        self.state = input.state;
        self.definition = input.definition;
        self.error = input.error;

        if state_changed {
            self.selection.reset(self.selectable_items().len());
        }
    }

    fn handle_event(&mut self, event: Self::Event) -> bool {
        let items = self.selectable_items();
        match event {
            KeyMouseEvent::Key(key) => match key.code {
                KeyCode::Char('j') | KeyCode::Down => {
                    self.selection.select_next(items.len());
                    true
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.selection.select_previous(items.len());
                    true
                }
                KeyCode::Char('i') => {
                    self.filters.toggle(FilterToggle::Instances);
                    true
                }
                KeyCode::Char('a') => {
                    self.filters.toggle(FilterToggle::Inaccessible);
                    true
                }
                KeyCode::Char('l') => {
                    self.filters.toggle(FilterToggle::LetValues);
                    true
                }
                KeyCode::Char('r') => {
                    self.filters.toggle(FilterToggle::ReverseOrder);
                    true
                }
                _ => false,
            },
            KeyMouseEvent::Mouse(mouse) => {
                if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
                    return false;
                }

                // Check for clicks on errors section
                if self
                    .errors_section_state
                    .handle_click(mouse.column, mouse.row)
                {
                    return true;
                }

                self.selection.handle_click(mouse.column, mouse.row, &items)
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.selection.clear_regions();

        let content_area = render_error(frame, area, self.error.as_deref());

        // Count total lines in error messages
        let error_lines: usize = self
            .current_node_errors
            .iter()
            .map(|e| e.message.lines().count())
            .sum();

        let (main_area, errors_area) = if error_lines > 0 {
            // Calculate error section height: content lines + 2 for borders
            // Cap at 1/3 of available space to leave room for content
            let max_error_height = (content_area.height / 3).max(3);
            let error_height = ((error_lines as u16) + 2).min(max_error_height);

            let [main, errors] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Length(error_height),
            ])
            .areas(content_area);
            (main, Some(errors))
        } else {
            (content_area, None)
        };

        let goal_list = OpenGoalList::new(
            &self.state,
            self.current_selection(),
            self.filters,
            self.current_node_id,
            self.active_goal_name.as_deref(),
        );
        let click_regions = goal_list.render_to_frame(frame, main_area);

        let y_offset = i32::from(main_area.y.saturating_sub(area.y));
        for region in click_regions {
            self.selection.add_region(
                Rect::new(
                    region.x as u16,
                    (region.y + y_offset) as u16,
                    region.width,
                    region.height,
                ),
                region.selection,
            );
        }

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

    const NAME: &'static str = "Plain list";
    const KEYBINDINGS: &'static [(&'static str, &'static str)] =
        &[("i", "inst"), ("a", "access"), ("l", "let"), ("r", "rev")];
    const SUPPORTED_FILTERS: &'static [FilterToggle] = &[
        FilterToggle::Instances,
        FilterToggle::Inaccessible,
        FilterToggle::LetValues,
        FilterToggle::ReverseOrder,
    ];

    fn current_selection(&self) -> Option<Selection> {
        self.selection
            .current_selection(&self.selectable_items())
            .copied()
    }
}
