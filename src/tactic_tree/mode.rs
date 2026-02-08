use std::collections::HashSet;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::Color,
    Frame,
};

use crate::{
    app::DefinitionInfo,
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
        FilterToggle, HypothesisFilters, InteractiveComponent, InteractiveStatefulWidget,
        KeyMouseEvent, ListSelection, Selection,
    },
    tactic_tree::{
        goal_section::{GoalSection, GoalSectionState},
        hyp_section::{HypSection, HypSectionState},
        proof_steps_sidebar::{ProofStepsSidebar, ProofStepsSidebarState},
    },
};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum FocusedPane {
    #[default]
    Sidebar,
    Hypotheses,
    Goals,
}

impl FocusedPane {
    pub const fn next(self) -> Self {
        match self {
            Self::Sidebar => Self::Hypotheses,
            Self::Hypotheses => Self::Goals,
            Self::Goals => Self::Sidebar,
        }
    }

    pub const fn prev(self) -> Self {
        match self {
            Self::Sidebar => Self::Goals,
            Self::Hypotheses => Self::Sidebar,
            Self::Goals => Self::Hypotheses,
        }
    }
}
/// Input for updating the Steps mode.
pub struct StepsModeInput {
    pub state: TacticProofState,
    pub definition: Option<DefinitionInfo>,
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
    definition: Option<DefinitionInfo>,
    error: Option<String>,
}

impl StepsModeInputBuilder {
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
            definition: self.definition,
            error: self.error,
            proof_dag: self.dag,
        }
    }
}

/// Steps display mode - sidebar + hypotheses + goals.
#[derive(Default)]
pub struct TacticTree {
    state: TacticProofState,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
    proof_dag: Option<CompleteProofDag>,
    /// Current node ID in the DAG (for building selections).
    current_node_id: Option<NodeId>,
    hyp_section_state: HypSectionState,
    errors_section_state: ErrorsSectionState,
    goal_section_state: GoalSectionState,
    sidebar_state: ProofStepsSidebarState,
    filters: HypothesisFilters,
    selection: ListSelection,
    /// Which pane is currently focused for keyboard navigation.
    focused_pane: FocusedPane,
    /// Last known sidebar viewport height (for scroll bounds).
    sidebar_viewport_height: usize,
}

impl TacticTree {
    fn selectable_items(&self) -> Vec<Selection> {
        let Some(node_id) = self.current_node_id else {
            return Vec::new();
        };

        // All hypotheses first, then all goals
        let hyps = hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order)
            .filter(|&i| {
                self.state.hypotheses.get(i).is_some_and(|h| {
                    (!self.filters.hide_instances || !h.is_typeclass_instance)
                        && (!self.filters.hide_inaccessible || !h.is_proof_term)
                })
            })
            .map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });

        let goals =
            (0..self.state.goals.len()).map(move |goal_idx| Selection::Goal { node_id, goal_idx });

        hyps.chain(goals).collect()
    }

    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    fn has_steps(&self) -> bool {
        self.proof_dag.as_ref().is_some_and(|dag| !dag.is_empty())
    }

    fn layout_with_sidebar(&self, area: Rect) -> (Rect, Option<Rect>) {
        if self.has_steps() {
            let [sidebar, main] = Layout::horizontal([
                Constraint::Ratio(3, 8), // Sidebar takes 3/8 of width
                Constraint::Ratio(5, 8), // Main content takes 5/8
            ])
            .areas(area);
            (main, Some(sidebar))
        } else {
            (area, None)
        }
    }

    fn layout_main(area: Rect, error_lines: usize) -> MainLayout {
        if error_lines > 0 {
            // Calculate error section height: content lines + 2 for borders
            // Cap at 1/3 of available space to leave room for hyps and goals
            let max_error_height = (area.height / 3).max(3);
            let error_height = ((error_lines as u16) + 2).min(max_error_height);

            let [hyps, errors, goals] = Layout::vertical([
                Constraint::Fill(1), // Hypotheses expand
                Constraint::Length(error_height),
                Constraint::Fill(1), // Goals expand equally
            ])
            .areas(area);
            MainLayout {
                hyps,
                errors: Some(errors),
                goals,
            }
        } else {
            let [hyps, goals] = Layout::vertical([
                Constraint::Fill(1), // Hypotheses expand
                Constraint::Fill(1), // Goals expand equally
            ])
            .areas(area);
            MainLayout {
                hyps,
                errors: None,
                goals,
            }
        }
    }

    fn current_node(&self) -> Option<&ProofDagTacticNode> {
        self.proof_dag
            .as_ref()
            .and_then(|dag| dag.current_node_id.and_then(|id| dag.get(id)))
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
        if self.handle_focus_switch(&key) {
            return true;
        }
        if self.handle_filter_toggle(key.code) {
            return true;
        }
        self.handle_navigation(key.code)
    }

    fn handle_focus_switch(&mut self, key: &KeyEvent) -> bool {
        let switched = match key.code {
            KeyCode::Tab if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.focused_pane = self.focused_pane.prev();
                true
            }
            KeyCode::Tab => {
                self.focused_pane = self.focused_pane.next();
                true
            }
            KeyCode::BackTab => {
                self.focused_pane = self.focused_pane.prev();
                true
            }
            _ => false,
        };
        if switched && self.focused_pane != FocusedPane::Sidebar {
            self.sidebar_state.reset_scroll();
        }
        switched
    }

    fn handle_filter_toggle(&mut self, code: KeyCode) -> bool {
        // Note: 'l' is only a filter toggle when NOT focused on Sidebar
        // (where it's used for horizontal scrolling)
        match code {
            KeyCode::Char('i') => {
                self.filters.toggle(FilterToggle::Instances);
                true
            }
            KeyCode::Char('a') => {
                self.filters.toggle(FilterToggle::Inaccessible);
                true
            }
            KeyCode::Char('l') if self.focused_pane != FocusedPane::Sidebar => {
                self.filters.toggle(FilterToggle::LetValues);
                true
            }
            KeyCode::Char('r') => {
                self.filters.toggle(FilterToggle::ReverseOrder);
                true
            }
            _ => false,
        }
    }

    fn handle_navigation(&mut self, code: KeyCode) -> bool {
        match self.focused_pane {
            FocusedPane::Sidebar => self.handle_sidebar_navigation(code),
            FocusedPane::Hypotheses | FocusedPane::Goals => self.handle_selection_navigation(code),
        }
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

    fn handle_selection_navigation(&mut self, code: KeyCode) -> bool {
        let items = self.selectable_items();
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.selection.select_next(items.len());
                true
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selection.select_previous(items.len());
                true
            }
            _ => false,
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> bool {
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

        let items = self.selectable_items();
        self.selection.handle_click(mouse.column, mouse.row, &items)
    }

    /// Take the clicked error message (if any) and clear it.
    pub fn take_clicked_error(&mut self) -> Option<String> {
        self.errors_section_state.take_clicked_error()
    }
}

struct MainLayout {
    hyps: Rect,
    errors: Option<Rect>,
    goals: Rect,
}

impl InteractiveComponent for TacticTree {
    type Input = StepsModeInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        let state_changed = self.state.goals.len() != input.state.goals.len()
            || self.state.hypotheses.len() != input.state.hypotheses.len();
        self.state = input.state;
        self.definition = input.definition;
        self.error = input.error;
        self.current_node_id = input.proof_dag.as_ref().and_then(|dag| dag.current_node_id);
        self.proof_dag = input.proof_dag;

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

        if self.state.goals.is_empty() {
            render_no_goals(frame, content);
            return;
        }

        let (main, sidebar) = self.layout_with_sidebar(content);

        if let Some(sidebar_area) = sidebar {
            ProofStepsSidebar::update_state(&mut self.sidebar_state, self.proof_dag.clone());
            self.sidebar_state
                .set_focused(self.focused_pane == FocusedPane::Sidebar);
            self.sidebar_viewport_height = sidebar_area.height.saturating_sub(2) as usize;
            frame.render_stateful_widget(ProofStepsSidebar, sidebar_area, &mut self.sidebar_state);
        }

        // Extract all data from current_node before any mutable self operations
        let (current_node_errors, depends_on, spawned_goal_ids, active_goal_name) = {
            let current_node = self.current_node();
            let errors = current_node
                .map(|node| node.diagnostics.clone())
                .unwrap_or_default();
            let depends_on: HashSet<String> = current_node
                .map(|node| node.hypothesis_dependencies.iter().cloned().collect())
                .unwrap_or_default();
            let spawned_goal_ids: HashSet<String> = current_node
                .map(|node| {
                    node.proof_state
                        .goals
                        .iter()
                        .filter(|g| !g.is_removed)
                        .filter_map(|g| g.username_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            let active_goal_name = current_node
                .and_then(|node| node.proof_state.goals.iter().find(|g| g.is_removed))
                .and_then(|g| g.username_str().map(String::from));
            (errors, depends_on, spawned_goal_ids, active_goal_name)
        };

        // Count total lines in error messages
        let error_lines: usize = current_node_errors
            .iter()
            .map(|e| e.message.lines().count())
            .sum();
        let layout = Self::layout_main(main, error_lines);

        self.hyp_section_state.update(
            &self.state,
            self.filters,
            depends_on,
            self.current_selection(),
            self.current_node_id,
        );
        self.hyp_section_state
            .set_focused(self.focused_pane == FocusedPane::Hypotheses);
        frame.render_stateful_widget(HypSection, layout.hyps, &mut self.hyp_section_state);
        for region in self.hyp_section_state.click_regions() {
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

        if let Some(errors_area) = layout.errors {
            self.errors_section_state.update(current_node_errors);
            frame.render_stateful_widget(
                ErrorsSection,
                errors_area,
                &mut self.errors_section_state,
            );
        }

        self.goal_section_state.update(
            &self.state,
            self.current_selection(),
            spawned_goal_ids,
            self.current_node_id,
            active_goal_name,
        );
        self.goal_section_state
            .set_focused(self.focused_pane == FocusedPane::Goals);
        frame.render_stateful_widget(GoalSection, layout.goals, &mut self.goal_section_state);
        for region in self.goal_section_state.click_regions() {
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
    }
}

impl Mode for TacticTree {
    type Model = StepsModeInput;
    const NAME: &'static str = "Tactic tree";
    const KEYBINDINGS: &'static [(&'static str, &'static str)] = &[("Tab", "pane")];
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
        vec![
            LegendSection::new(
                "Tactic Tree",
                Color::Yellow,
                vec![
                    LegendItem::new("▶", Theme::CURRENT_NODE_BORDER, "current step"),
                    LegendItem::new("⊢", Theme::GOAL, "open goal"),
                    LegendItem::new("uses", Theme::DEPENDENCY_FG, "dependency"),
                ],
            ),
            LegendSection::new(
                "Hypotheses",
                Theme::TITLE_ASSUMPTION.into(),
                vec![LegendItem::new("h", Theme::HYP_NAME, "hypothesis")],
            ),
        ]
    }
}
