use std::collections::HashSet;

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
    data_flow::{
        layout::{FunctionalTableauLayout, FunctionalTableauState},
        types::DataFlowDag,
    },
    display_mode::Mode,
    shared::{
        find_nearest_in_direction, key_to_direction,
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
    pub dag: Option<DataFlowDag>,
}

impl DataFlowModeInput {
    /// Create a builder for this input type.
    pub fn builder() -> DataFlowModeInputBuilder {
        DataFlowModeInputBuilder::default()
    }
}

/// Builder for `DataFlowModeInput`.
#[derive(Default)]
pub struct DataFlowModeInputBuilder {
    dag: Option<DataFlowDag>,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
}

impl DataFlowModeInputBuilder {
    pub fn dag(mut self, dag: &DataFlowDag) -> Self {
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

    pub fn build(self) -> DataFlowModeInput {
        DataFlowModeInput {
            definition: self.definition,
            error: self.error,
            dag: self.dag,
        }
    }
}

/// Data Flow display mode - functional DAG visualization.
pub struct DataFlowMode {
    definition: Option<DefinitionInfo>,
    error: Option<String>,
    dag: Option<DataFlowDag>,
    filters: HypothesisFilters,
    /// Flat index into `tree_selectable_items()`.
    selected_idx: Option<usize>,
    /// When true, tree renders top-down (parent above children).
    tree_top_down: bool,
    /// State for the functional tableau widget.
    functional_tableau_state: FunctionalTableauState,
    /// Clicked error message to display in banner.
    clicked_error: Option<String>,
}

impl Default for DataFlowMode {
    fn default() -> Self {
        Self {
            definition: None,
            error: None,
            dag: None,
            filters: HypothesisFilters::default(),
            selected_idx: None,
            tree_top_down: true,
            functional_tableau_state: FunctionalTableauState::default(),
            clicked_error: None,
        }
    }
}

impl DataFlowMode {
    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    /// Take the clicked error message (if any) and clear it.
    pub fn take_clicked_error(&mut self) -> Option<String> {
        self.clicked_error.take()
    }

    /// Build selectable items for data flow DAG.
    fn tree_selectable_items(&self) -> Vec<Selection> {
        let Some(dag) = &self.dag else {
            return Vec::new();
        };

        if dag.is_empty() {
            return Vec::new();
        }

        let initial_hyps = dag
            .block_input
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.is_implicit && !b.is_instance)
            .map(|(idx, _)| Selection::InitialHyp { hyp_idx: idx });

        let node_selections = dag.dfs_iter().flat_map(|node| {
            let node_id = node.id;
            let binding_count = node.bindings_after().len();
            (0..binding_count)
                .map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx })
                .chain(std::iter::once(Selection::Goal {
                    node_id,
                    goal_idx: 0,
                }))
        });

        initial_hyps
            .chain(node_selections)
            .chain(std::iter::once(Selection::Theorem))
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
        let dag = self.dag.as_ref()?;
        let current_node_id = dag.current_node_id?;
        Some(Selection::Goal {
            node_id: current_node_id,
            goal_idx: 0,
        })
    }

    /// Collect unique node kinds as legend items.
    fn collect_kind_legend_items(&self) -> Vec<LegendItem> {
        let Some(dag) = &self.dag else {
            return Vec::new();
        };

        let mut seen = HashSet::new();
        dag.nodes
            .iter()
            .filter_map(|node| {
                let kind = node.data_flow_kind();
                let desc = kind.description().to_owned();
                seen.insert(desc.clone())
                    .then(|| LegendItem::new(kind.icon(), kind.color(), desc))
            })
            .collect()
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
        let navigation_regions = self.functional_tableau_state.navigation_regions();

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

impl InteractiveComponent for DataFlowMode {
    type Input = DataFlowModeInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        let old_count = self.dag.as_ref().map_or(0, DataFlowDag::len);
        let new_count = input.dag.as_ref().map_or(0, DataFlowDag::len);

        let old_current = self.dag.as_ref().and_then(|dag| dag.current_node_id);
        let new_current = input.dag.as_ref().and_then(|dag| dag.current_node_id);

        let tree_changed = old_count != new_count || old_current != new_current;

        self.functional_tableau_state
            .update_current_node(input.dag.as_ref().and_then(|dag| dag.current_node_id));

        self.definition = input.definition;
        self.error = input.error;
        self.dag = input.dag;

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

                let clicked = self
                    .functional_tableau_state
                    .find_click_at(mouse.column, mouse.row);

                if let Some(sel) = clicked {
                    // Handle error clicks specially - show full error in banner
                    if let Selection::Error { node_id, error_idx } = sel {
                        if let Some(dag) = &self.dag {
                            if let Some(node) = dag.get(node_id) {
                                if let Some(error) = node.diagnostics.get(error_idx) {
                                    self.clicked_error = Some(error.message.clone());
                                    return true;
                                }
                            }
                        }
                    }
                    self.select_by_selection(sel);
                    return true;
                }
                false
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let content_area = render_error(frame, area, self.error.as_deref());

        if let Some(ref dag) = self.dag {
            let widget = FunctionalTableauLayout::new(
                dag,
                self.tree_top_down,
                self.current_tree_selection(),
            );
            frame.render_stateful_widget(widget, content_area, &mut self.functional_tableau_state);
            return;
        }

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
        let kind_items = self.collect_kind_legend_items();

        vec![
            LegendSection::new("Node Kinds", Color::White, kind_items),
            LegendSection::new(
                "Node States",
                Color::White,
                vec![
                    LegendItem::new("■", Theme::CURRENT_NODE_BORDER, "current"),
                    LegendItem::new("■", Theme::COMPLETED_GOAL_FG, "complete"),
                    LegendItem::new("■", Theme::INCOMPLETE_NODE_BORDER, "incomplete"),
                ],
            ),
        ]
    }
}
