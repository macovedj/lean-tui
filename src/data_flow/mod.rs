pub mod graph;
pub mod render;

use std::{collections::HashSet, iter::once};

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::Paragraph,
    Frame,
};
use tracing::trace;
pub use graph::{DataFlowConversionError, DataFlowDag};

use crate::{
    data_flow::render::DataFlowLayout,
    graph_layout::GraphViewport,
    interactive::Mode,
    navigation::{find_nearest_in_direction, key_to_direction, Direction},
    selection::Selection,
    theme::Theme,
    widgets::{
        legend::{LegendItem, LegendSection},
        render_helpers::render_error,
    },
};

/// Data Flow display mode - functional DAG visualization.
pub struct DataFlowMode {
    error: Option<String>,
    dag: Option<DataFlowDag>,
    /// Flat index into `tree_selectable_items()`.
    selected_idx: Option<usize>,
    /// When true, tree renders top-down (parent above children).
    tree_top_down: bool,
    viewport: GraphViewport,
    /// Clicked error message to display in banner.
    clicked_error: Option<String>,
}

impl Default for DataFlowMode {
    fn default() -> Self {
        Self {
            error: None,
            dag: None,
            selected_idx: None,
            tree_top_down: true,
            viewport: GraphViewport::default(),
            clicked_error: None,
        }
    }
}

impl DataFlowMode {
    fn get_error_message(&self, node_id: u64, error_idx: usize) -> Option<String> {
        self.dag
            .as_ref()
            .and_then(|dag| dag.get(node_id))
            .and_then(|node| node.diagnostics.get(error_idx))
            .map(|e| e.message.clone())
    }

    fn handle_error_click(&mut self, sel: Selection) -> bool {
        let Selection::Error { node_id, error_idx } = sel else {
            return false;
        };
        self.get_error_message(node_id, error_idx)
            .map(|msg| self.clicked_error = Some(msg))
            .is_some()
    }

    /// Build selectable items for data flow DAG.
    fn tree_selectable_items(&self) -> Vec<Selection> {
        let Some(dag) = &self.dag else {
            return Vec::new();
        };

        if dag.is_empty() {
            return Vec::new();
        }

        // Input/output panes are now global - only include node-specific selections
        dag.dfs_iter()
            .flat_map(|node| {
                let node_id = node.id;
                let binding_count = node
                    .bindings()
                    .iter()
                    .filter(|b| !b.is_implicit && !b.is_instance)
                    .count();
                let hyp_selections =
                    (0..binding_count).map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });
                let goal_selection = once(Selection::Goal {
                    node_id,
                    goal_idx: 0,
                });
                hyp_selections.chain(goal_selection)
            })
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
        let navigation_regions = self.viewport.navigation_regions();

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

impl Mode for DataFlowMode {
    type Dag = DataFlowDag;

    fn name(&self) -> &'static str {
        "Data Flow"
    }

    fn keybindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("hjkl", "nav"), ("t", "flip")]
    }

    fn current_selection(&self) -> Option<Selection> {
        self.current_tree_selection()
    }

    fn take_clicked_error(&mut self) -> Option<String> {
        self.clicked_error.take()
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

    fn update(&mut self, dag: Option<&Self::Dag>, error: Option<String>) {
        let old_count = self.dag.as_ref().map_or(0, DataFlowDag::len);
        let new_count = dag.map_or(0, DataFlowDag::len);

        let old_current = self.dag.as_ref().and_then(|dag| dag.current_node_id);
        let new_current = dag.and_then(|dag| dag.current_node_id);

        let tree_changed = old_count != new_count || old_current != new_current;

        self.viewport
            .update_current_node(dag.and_then(|dag| dag.current_node_id));

        self.error = error;
        self.dag = dag.cloned();

        if tree_changed {
            if let Some(sel) = self.active_goal_selection() {
                self.select_by_selection(sel);
            } else {
                self.selected_idx = None;
            }
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> bool {
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

    fn handle_mouse(&mut self, mouse: MouseEvent) -> bool {
        if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
            return false;
        }

        let clicked = self.viewport.find_click_at(mouse.column, mouse.row);

        let Some(sel) = clicked else {
            return false;
        };

        if self.handle_error_click(sel) {
            return true;
        }

        self.select_by_selection(sel);
        true
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let content_area = render_error(frame, area, self.error.as_deref());

        if let Some(ref dag) = self.dag {
            let widget =
                DataFlowLayout::new(dag, self.tree_top_down, self.current_tree_selection());
            frame.render_stateful_widget(widget, content_area, &mut self.viewport);
            return;
        }

        frame.render_widget(
            Paragraph::new("No data flow available (term mode only)")
                .style(Style::new().fg(Color::DarkGray)),
            content_area,
        );
    }
}
