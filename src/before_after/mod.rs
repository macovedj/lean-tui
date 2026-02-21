mod diff_pane;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use diff_pane::{DiffPane, DiffPaneKind, DiffPaneState};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Color,
    Frame,
};

use crate::{
    deduction_tree::{graph::CompleteProofDag, TacticProofState},
    interactive::Mode,
    protocol::{DiagnosticInfo, NodeId},
    selection::{ListSelection, Selection},
    theme::Theme,
    widgets::{
        errors_section::{ErrorsSection, ErrorsSectionState},
        legend::{LegendItem, LegendSection},
        render_helpers::render_error,
    },
};

pub struct BeforeAfterMode {
    previous_state: Option<TacticProofState>,
    current_state: TacticProofState,
    next_state: Option<TacticProofState>,
    error: Option<String>,
    current_node_id: Option<NodeId>,
    active_goal_name: Option<String>,
    selection: ListSelection,
    show_previous: bool,
    show_next: bool,
    before_pane_state: DiffPaneState,
    current_pane_state: DiffPaneState,
    after_pane_state: DiffPaneState,
    current_node_errors: Vec<DiagnosticInfo>,
    errors_section_state: ErrorsSectionState,
}

impl Default for BeforeAfterMode {
    fn default() -> Self {
        Self {
            previous_state: None,
            current_state: TacticProofState::default(),
            next_state: None,
            error: None,
            current_node_id: None,
            active_goal_name: None,
            selection: ListSelection::default(),
            show_previous: true,
            show_next: false,
            before_pane_state: DiffPaneState::default(),
            current_pane_state: DiffPaneState::default(),
            after_pane_state: DiffPaneState::default(),
            current_node_errors: Vec::new(),
            errors_section_state: ErrorsSectionState::default(),
        }
    }
}

impl BeforeAfterMode {
    fn selectable_items(&self) -> Vec<Selection> {
        let Some(node_id) = self.current_node_id else {
            return Vec::new();
        };

        let hyp_items = (0..self.current_state.hypotheses.len())
            .map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });

        let goal_items = (0..self.current_state.goals.len())
            .map(move |goal_idx| Selection::Goal { node_id, goal_idx });

        hyp_items.chain(goal_items).collect()
    }
}

impl Mode for BeforeAfterMode {
    type Dag = CompleteProofDag;

    fn name(&self) -> &'static str {
        "Compact"
    }

    fn keybindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("p", "prev"), ("n", "next")]
    }

    fn current_selection(&self) -> Option<Selection> {
        self.selection
            .current_selection(&self.selectable_items())
            .copied()
    }

    fn take_clicked_error(&mut self) -> Option<String> {
        self.errors_section_state.take_clicked_error()
    }

    fn legend_sections(&self) -> Vec<LegendSection> {
        vec![
            LegendSection::new(
                "Diff markers",
                Color::White,
                vec![
                    LegendItem::new("x", Theme::UNCHANGED, "unchanged"),
                    LegendItem::new("+", Theme::ADDED, "added"),
                    LegendItem::new("~", Theme::CHANGED, "changed"),
                    LegendItem::new("-", Theme::REMOVED, "removed"),
                ],
            ),
            LegendSection::new(
                "Elements",
                Color::White,
                vec![
                    LegendItem::new("h", Theme::HYP_NAME, "hypothesis"),
                    LegendItem::new("⊢", Theme::GOAL, "goal"),
                ],
            ),
        ]
    }

    fn update(&mut self, dag: Option<&Self::Dag>, error: Option<String>) {
        let current_node_id = dag.and_then(|dag| dag.current_node_id);
        let current_node = current_node_id.and_then(|id| dag?.get(id));

        let new_current_state = current_node
            .map(|n| n.proof_state.clone())
            .unwrap_or_default();

        let state_changed = self.current_state.goals.len() != new_current_state.goals.len()
            || self.current_state.hypotheses.len() != new_current_state.hypotheses.len();

        self.previous_state = self
            .show_previous
            .then(|| {
                current_node
                    .and_then(|n| n.parent)
                    .and_then(|parent_id| dag?.get(parent_id))
                    .map(|parent| parent.proof_state.clone())
            })
            .flatten();

        self.current_state = new_current_state;

        self.next_state = self
            .show_next
            .then(|| {
                current_node
                    .and_then(|n| n.children.first())
                    .and_then(|&child_id| dag?.get(child_id))
                    .map(|child| child.proof_state.clone())
            })
            .flatten();

        self.error = error;
        self.current_node_id = current_node_id;
        self.active_goal_name = current_node
            .and_then(|node| node.proof_state.goals.first())
            .and_then(|g| g.username_str().map(String::from));
        self.current_node_errors = current_node
            .map(|node| node.diagnostics.clone())
            .unwrap_or_default();

        if state_changed {
            self.selection.reset(self.selectable_items().len());
        }
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
            KeyCode::Char('p') => {
                self.show_previous = !self.show_previous;
                true
            }
            KeyCode::Char('n') => {
                self.show_next = !self.show_next;
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

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        self.selection.clear_regions();

        let content_area = render_error(frame, area, self.error.as_deref());

        let error_lines: usize = self
            .current_node_errors
            .iter()
            .map(|e| e.message.lines().count())
            .sum();

        let (main_area, errors_area) = if error_lines > 0 {
            let error_height = ((error_lines as u16) + 2).min(content_area.height / 2);

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
                    DiffPane::new(
                        DiffPaneKind::Before,
                        state,
                        None,
                        None,
                        self.active_goal_name.as_deref(),
                    ),
                    columns[col_idx],
                    &mut self.before_pane_state,
                );
                col_idx += 1;
            }
        }

        frame.render_stateful_widget(
            DiffPane::new(
                DiffPaneKind::Current,
                &self.current_state,
                selection,
                self.current_node_id,
                self.active_goal_name.as_deref(),
            ),
            columns[col_idx],
            &mut self.current_pane_state,
        );

        for region in self.current_pane_state.click_regions() {
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
                    DiffPane::new(
                        DiffPaneKind::After,
                        state,
                        None,
                        None,
                        self.active_goal_name.as_deref(),
                    ),
                    columns[col_idx],
                    &mut self.after_pane_state,
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
