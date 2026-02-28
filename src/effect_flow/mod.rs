pub mod graph;
mod render;

use std::iter::once;

use crossterm::event::{KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
pub use graph::{EdgeKind, EffectFlowConversionError, EffectFlowDag, EffectKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use tracing::trace;

use crate::{
    effect_flow::render::EffectFlowLayout,
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

#[derive(Default)]
pub struct EffectFlowMode {
    error: Option<String>,
    effect_dag: Option<EffectFlowDag>,
    selected_idx: Option<usize>,
    pub(crate) viewport: GraphViewport,
    expanded: bool,
}

impl EffectFlowMode {
    fn tree_selectable_items(&self) -> Vec<Selection> {
        let Some(dag) = &self.effect_dag else {
            return Vec::new();
        };

        if dag.is_empty() {
            return Vec::new();
        }

        dag.dfs_iter()
            .flat_map(|node| {
                let node_id = node.id;
                let binding_count = node.visible_bindings().len();
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

    fn active_goal_selection(&self) -> Option<Selection> {
        let dag = self.effect_dag.as_ref()?;
        let current_node_id = dag.current_node_id?;
        Some(Selection::Goal {
            node_id: current_node_id,
            goal_idx: 0,
        })
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

    /// Render the monad stack info at the top.
    fn render_monad_stack(&self, frame: &mut Frame, area: Rect) -> Rect {
        let Some(dag) = &self.effect_dag else {
            return area;
        };

        let Some(monad_stack) = dag.monad_stack() else {
            return area;
        };

        if monad_stack.is_empty() {
            return area;
        }

        let spans = vec![
            Span::styled("Monad: ", Style::new().fg(Color::DarkGray)),
            Span::styled(
                monad_stack.join(" → "),
                Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ),
        ];
        frame.render_widget(Paragraph::new(Line::from(spans)), area);

        if area.height > 1 {
            Rect::new(area.x, area.y + 1, area.width, area.height - 1)
        } else {
            area
        }
    }
}

impl Mode for EffectFlowMode {
    type Dag = EffectFlowDag;

    fn name(&self) -> &'static str {
        "Effect Flow"
    }

    fn keybindings(&self) -> &'static [(&'static str, &'static str)] {
        &[("hjkl", "nav"), ("e", "expand")]
    }

    fn current_selection(&self) -> Option<Selection> {
        self.current_tree_selection()
    }

    fn legend_sections(&self) -> Vec<LegendSection> {
        vec![
            LegendSection::new(
                "Nodes",
                Color::White,
                vec![
                    LegendItem::new("■", Theme::CURRENT_NODE_BORDER, "current"),
                    LegendItem::new(EffectKind::EffectBind.icon(), Theme::EFFECT_BIND, "bind"),
                    LegendItem::new(
                        EffectKind::StateRead.icon(),
                        Theme::EFFECT_STATE_READ,
                        "read",
                    ),
                    LegendItem::new(
                        EffectKind::StateWrite.icon(),
                        Theme::EFFECT_STATE_WRITE,
                        "write",
                    ),
                    LegendItem::new(
                        EffectKind::StateModify.icon(),
                        Theme::EFFECT_STATE_MODIFY,
                        "modify",
                    ),
                    LegendItem::new(
                        EffectKind::EffectReturn.icon(),
                        Theme::EFFECT_RETURN,
                        "return",
                    ),
                    LegendItem::new(EffectKind::EffectThrow.icon(), Theme::EFFECT_THROW, "throw"),
                    LegendItem::new(EffectKind::Pure.icon(), Theme::EFFECT_PURE, "pure"),
                ],
            ),
            LegendSection::new(
                "Edges",
                Color::White,
                vec![
                    LegendItem::new("─", EdgeKind::DataFlow.color(), "data"),
                    LegendItem::new("─", EdgeKind::ControlFlow.color(), "control"),
                    LegendItem::new("─", EdgeKind::EffectFlow.color(), "effect"),
                    LegendItem::new("─", EdgeKind::ErrorPropagation.color(), "error"),
                    LegendItem::new("─", EdgeKind::EarlyReturn.color(), "early return"),
                    LegendItem::new("─", EdgeKind::Continuation.color(), "continuation"),
                ],
            ),
        ]
    }

    fn update(&mut self, dag: Option<&Self::Dag>, error: Option<String>) {
        let old_count = self.effect_dag.as_ref().map_or(0, EffectFlowDag::len);
        let new_count = dag.map_or(0, EffectFlowDag::len);

        if let Some(dag) = dag {
            trace!(
                node_count = dag.len(),
                root_id = ?dag.root_node_id,
                current_id = ?dag.current_node_id,
                "EffectFlowMode received DAG"
            );
            for node in dag.dfs_iter() {
                trace!(
                    node_id = node.id,
                    kind = ?node.effect_kind,
                    edges = node.edges.len(),
                    content = %node.content.to_plain_text(),
                    "  Node"
                );
            }
        }

        let old_current = self.effect_dag.as_ref().and_then(|dag| dag.current_node_id);
        let new_current = dag.and_then(|dag| dag.current_node_id);

        let tree_changed = old_count != new_count || old_current != new_current;

        self.viewport
            .update_current_node(dag.and_then(|dag| dag.current_node_id));

        self.error = error;
        self.effect_dag = dag.cloned();

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
            KeyCode::Char('e') => {
                self.expanded = !self.expanded;
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

        if let Some(sel) = clicked {
            self.select_by_selection(sel);
            return true;
        }
        false
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let content_area = render_error(frame, area, self.error.as_deref());

        if let Some(ref dag) = self.effect_dag {
            let remaining = self.render_monad_stack(frame, content_area);
            let widget = EffectFlowLayout::new(dag, self.current_tree_selection(), self.expanded);
            frame.render_stateful_widget(widget, remaining, &mut self.viewport);
            return;
        }

        frame.render_widget(
            Paragraph::new("No effect flow DAG available").style(Style::new().fg(Color::DarkGray)),
            content_area,
        );
    }
}
