use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Color,
    Frame,
};

use crate::{
    app::DefinitionInfo,
    before_after::goals_column::{GoalsColumn, GoalsColumnState},
    deduction_tree::{types::CompleteProofDag, TacticProofState},
    display_mode::Mode,
    protocol::{DiagnosticInfo, NodeId},
    shared::{
        errors_section::{ErrorsSection, ErrorsSectionState},
        hypothesis_indices,
        legend::{LegendItem, LegendSection},
        render_helpers::render_error,
        theme::Theme,
        FilterToggle, HypothesisFilters, InteractiveComponent, KeyMouseEvent, ListSelection,
        Selection,
    },
};

/// Input for updating the Before/After mode.
pub struct BeforeAfterModeInput {
    pub previous_state: Option<TacticProofState>,
    pub current_state: TacticProofState,
    pub next_state: Option<TacticProofState>,
    pub definition: Option<DefinitionInfo>,
    pub error: Option<String>,
    pub proof_dag: Option<CompleteProofDag>,
}

impl BeforeAfterModeInput {
    /// Create a builder for this input type.
    pub fn builder() -> BeforeAfterModeInputBuilder {
        BeforeAfterModeInputBuilder::default()
    }
}

/// Builder for `BeforeAfterModeInput`.
#[derive(Default)]
pub struct BeforeAfterModeInputBuilder {
    dag: Option<CompleteProofDag>,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
    show_previous: bool,
    show_next: bool,
}

impl BeforeAfterModeInputBuilder {
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

    pub fn show_previous(mut self, show: bool) -> Self {
        self.show_previous = show;
        self
    }

    pub fn show_next(mut self, show: bool) -> Self {
        self.show_next = show;
        self
    }

    pub fn build(self) -> BeforeAfterModeInput {
        let current_node = self
            .dag
            .as_ref()
            .and_then(|d| d.current_node_id)
            .and_then(|id| self.dag.as_ref()?.get(id));

        let current_state = current_node
            .map(|n| n.proof_state_after.clone())
            .unwrap_or_default();

        let previous_state = self
            .show_previous
            .then(|| current_node.map(|n| n.proof_state_before.clone()))
            .flatten();

        let next_state = self
            .show_next
            .then(|| {
                current_node
                    .and_then(|n| n.children.first())
                    .and_then(|&child_id| self.dag.as_ref()?.get(child_id))
                    .map(|child| child.proof_state_after.clone())
            })
            .flatten();

        BeforeAfterModeInput {
            previous_state,
            current_state,
            next_state,
            definition: self.definition,
            error: self.error,
            proof_dag: self.dag,
        }
    }
}

/// Before/After display mode - temporal comparison of goal states.
pub struct BeforeAfterMode {
    previous_state: Option<TacticProofState>,
    current_state: TacticProofState,
    next_state: Option<TacticProofState>,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
    /// Current node ID in the DAG (for building selections).
    current_node_id: Option<NodeId>,
    /// Name of the goal the cursor's tactic is working on.
    active_goal_name: Option<String>,
    filters: HypothesisFilters,
    selection: ListSelection,
    show_previous: bool,
    show_next: bool,
    previous_column_state: GoalsColumnState,
    current_column_state: GoalsColumnState,
    next_column_state: GoalsColumnState,
    /// Errors from the current node.
    current_node_errors: Vec<DiagnosticInfo>,
    errors_section_state: ErrorsSectionState,
}

impl Default for BeforeAfterMode {
    fn default() -> Self {
        Self {
            previous_state: None,
            current_state: TacticProofState::default(),
            next_state: None,
            definition: None,
            error: None,
            current_node_id: None,
            active_goal_name: None,
            filters: HypothesisFilters::default(),
            selection: ListSelection::default(),
            show_previous: true,
            show_next: false,
            previous_column_state: GoalsColumnState::default(),
            current_column_state: GoalsColumnState::default(),
            next_column_state: GoalsColumnState::default(),
            current_node_errors: Vec::new(),
            errors_section_state: ErrorsSectionState::default(),
        }
    }
}

impl BeforeAfterMode {
    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    /// Whether the previous column should be shown.
    pub const fn show_previous(&self) -> bool {
        self.show_previous
    }

    /// Whether the next column should be shown.
    pub const fn show_next(&self) -> bool {
        self.show_next
    }

    fn selectable_items(&self) -> Vec<Selection> {
        let Some(node_id) = self.current_node_id else {
            return Vec::new();
        };

        let hyp_items = hypothesis_indices(
            self.current_state.hypotheses.len(),
            self.filters.reverse_order,
        )
        .filter(|&hyp_idx| {
            self.current_state.hypotheses.get(hyp_idx).is_some_and(|h| {
                (!self.filters.hide_instances || !h.is_typeclass_instance)
                    && (!self.filters.hide_inaccessible || !h.is_proof_term)
            })
        })
        .map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });

        let goal_items = (0..self.current_state.goals.len())
            .map(move |goal_idx| Selection::Goal { node_id, goal_idx });

        hyp_items.chain(goal_items).collect()
    }

    /// Take the clicked error message (if any) and clear it.
    pub fn take_clicked_error(&mut self) -> Option<String> {
        self.errors_section_state.take_clicked_error()
    }
}

impl InteractiveComponent for BeforeAfterMode {
    type Input = BeforeAfterModeInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        let state_changed = self.current_state.goals.len() != input.current_state.goals.len()
            || self.current_state.hypotheses.len() != input.current_state.hypotheses.len();
        self.previous_state = input.previous_state;
        self.current_state = input.current_state;
        self.next_state = input.next_state;
        self.definition = input.definition;
        self.error = input.error;
        let current_node_id = input.proof_dag.as_ref().and_then(|dag| dag.current_node_id);
        let current_node = current_node_id.and_then(|id| input.proof_dag.as_ref()?.get(id));
        self.current_node_id = current_node_id;
        self.active_goal_name = current_node
            .and_then(|node| node.proof_state_before.goals.first())
            .and_then(|g| g.username_str().map(String::from));
        self.current_node_errors = current_node
            .map(|node| node.diagnostics.clone())
            .unwrap_or_default();
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
                KeyCode::Char('p') => {
                    self.show_previous = !self.show_previous;
                    true
                }
                KeyCode::Char('n') => {
                    self.show_next = !self.show_next;
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

    #[allow(clippy::too_many_lines)]
    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.selection.clear_regions();

        let content_area = render_error(frame, area, self.error.as_deref());

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

            let [main, errors] =
                Layout::vertical([Constraint::Fill(1), Constraint::Length(error_height)])
                    .areas(content_area);
            (main, Some(errors))
        } else {
            (content_area, None)
        };

        let has_prev = self.previous_state.is_some() && self.show_previous;
        let has_next = self.next_state.is_some() && self.show_next;

        let constraints = match (has_prev, has_next) {
            (true, true) => vec![
                Constraint::Percentage(30),
                Constraint::Percentage(40),
                Constraint::Percentage(30),
            ],
            (true, false) => vec![Constraint::Percentage(35), Constraint::Percentage(65)],
            (false, true) => vec![Constraint::Percentage(65), Constraint::Percentage(35)],
            (false, false) => vec![Constraint::Percentage(100)],
        };

        let columns = Layout::horizontal(constraints).split(main_area);
        let mut col_idx = 0;

        let selection = self.current_selection();

        if let Some(ref state) = self.previous_state {
            if self.show_previous {
                frame.render_stateful_widget(
                    GoalsColumn::new(
                        "Previous",
                        state,
                        self.filters,
                        None,
                        false,
                        None,
                        self.active_goal_name.as_deref(),
                    ),
                    columns[col_idx],
                    &mut self.previous_column_state,
                );
                col_idx += 1;
            }
        }

        frame.render_stateful_widget(
            GoalsColumn::new(
                "Current",
                &self.current_state,
                self.filters,
                selection,
                true,
                self.current_node_id,
                self.active_goal_name.as_deref(),
            ),
            columns[col_idx],
            &mut self.current_column_state,
        );
        for region in self.current_column_state.click_regions() {
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
        col_idx += 1;

        if let Some(ref state) = self.next_state {
            if self.show_next {
                frame.render_stateful_widget(
                    GoalsColumn::new(
                        "Next",
                        state,
                        self.filters,
                        None,
                        false,
                        None,
                        self.active_goal_name.as_deref(),
                    ),
                    columns[col_idx],
                    &mut self.next_column_state,
                );
            }
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

impl Mode for BeforeAfterMode {
    type Model = BeforeAfterModeInput;

    const NAME: &'static str = "Before/After";
    const KEYBINDINGS: &'static [(&'static str, &'static str)] = &[
        ("p", "prev"),
        ("n", "next"),
        ("i", "inst"),
        ("a", "access"),
        ("l", "let"),
        ("r", "rev"),
    ];
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

    fn legend_sections(&self) -> Vec<LegendSection> {
        vec![LegendSection::new(
            "Diff",
            Color::White,
            vec![
                LegendItem::new("■", Theme::CURRENT_NODE_BORDER, "current"),
                LegendItem::new("h", Theme::HYP_NAME, "hypothesis"),
                LegendItem::new("+", Theme::ADDED, "added"),
                LegendItem::new("~", Theme::CHANGED, "changed"),
                LegendItem::new("-", Theme::REMOVED, "removed"),
            ],
        )]
    }
}
