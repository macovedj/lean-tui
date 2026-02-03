use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};
use tracing::trace;

use crate::{
    app::DefinitionInfo,
    display_mode::Mode,
    effect_flow::{
        layout::{EffectTableauLayout, EffectTableauState},
        types::{EdgeKind, EffectFlowDag, EffectKind},
    },
    shared::{
        find_nearest_in_direction,
        legend::{LegendItem, LegendSection},
        render_helpers::render_error,
        Direction, FilterToggle, HypothesisFilters, InteractiveComponent, KeyMouseEvent, Selection,
        Theme,
    },
};

/// Input for updating the Effect Flow mode.
pub struct EffectFlowModeInput {
    pub definition: Option<DefinitionInfo>,
    pub error: Option<String>,
    pub effect_dag: Option<EffectFlowDag>,
}

impl EffectFlowModeInput {
    /// Create a builder for this input type.
    pub fn builder() -> EffectFlowModeInputBuilder {
        EffectFlowModeInputBuilder::default()
    }
}

/// Builder for `EffectFlowModeInput`.
#[derive(Default)]
pub struct EffectFlowModeInputBuilder {
    effect_dag: Option<EffectFlowDag>,
    definition: Option<DefinitionInfo>,
    error: Option<String>,
}

impl EffectFlowModeInputBuilder {
    pub fn effect_dag(mut self, dag: &EffectFlowDag) -> Self {
        self.effect_dag = Some(dag.clone());
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

    pub fn build(self) -> EffectFlowModeInput {
        EffectFlowModeInput {
            definition: self.definition,
            error: self.error,
            effect_dag: self.effect_dag,
        }
    }
}

pub struct EffectFlowMode {
    definition: Option<DefinitionInfo>,
    error: Option<String>,
    effect_dag: Option<EffectFlowDag>,
    filters: HypothesisFilters,
    selected_idx: Option<usize>,
    tree_top_down: bool,
    show_interpretations: bool,
    effect_tableau_state: EffectTableauState,
}

impl Default for EffectFlowMode {
    fn default() -> Self {
        Self {
            definition: None,
            error: None,
            effect_dag: None,
            filters: HypothesisFilters::default(),
            selected_idx: None,
            tree_top_down: true,
            show_interpretations: true,
            effect_tableau_state: EffectTableauState::default(),
        }
    }
}

impl EffectFlowMode {
    pub const fn filters(&self) -> HypothesisFilters {
        self.filters
    }

    fn tree_selectable_items(&self) -> Vec<Selection> {
        let Some(dag) = &self.effect_dag else {
            return Vec::new();
        };

        if dag.is_empty() {
            return Vec::new();
        }

        let mut items = Vec::new();

        // Initial bindings
        for (idx, b) in dag.initial_context.iter().enumerate() {
            if !b.is_implicit && !b.is_instance {
                items.push(Selection::InitialHyp { hyp_idx: idx });
            }
        }

        // Effect nodes
        for node in dag.dfs_iter() {
            items.push(Selection::Goal {
                node_id: node.id,
                goal_idx: 0,
            });
        }

        // Definition type
        items.push(Selection::Theorem);

        items
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
        let navigation_regions = self.effect_tableau_state.navigation_regions();

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

        // Return remaining area below the monad stack line
        if area.height > 1 {
            Rect::new(area.x, area.y + 1, area.width, area.height - 1)
        } else {
            area
        }
    }
}

impl InteractiveComponent for EffectFlowMode {
    type Input = EffectFlowModeInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        let old_count = self.effect_dag.as_ref().map_or(0, EffectFlowDag::len);
        let new_count = input.effect_dag.as_ref().map_or(0, EffectFlowDag::len);

        if let Some(ref dag) = input.effect_dag {
            trace!(
                node_count = dag.len(),
                root_id = ?dag.root_node_id,
                current_id = ?dag.current_node_id,
                "EffectFlowMode received DAG"
            );
            for node in dag.dfs_iter() {
                trace!(
                    node_id = node.id,
                    kind = ?node.kind,
                    edges = node.edges.len(),
                    content = %node.content.to_plain_text(),
                    "  Node"
                );
            }
        }

        let old_current = self.effect_dag.as_ref().and_then(|dag| dag.current_node_id);
        let new_current = input
            .effect_dag
            .as_ref()
            .and_then(|dag| dag.current_node_id);

        let tree_changed = old_count != new_count || old_current != new_current;

        self.effect_tableau_state.update_current_node(
            input
                .effect_dag
                .as_ref()
                .and_then(|dag| dag.current_node_id),
        );

        self.definition = input.definition;
        self.error = input.error;
        self.effect_dag = input.effect_dag;

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
            KeyMouseEvent::Key(key) => match key.code {
                KeyCode::Char('j') | KeyCode::Down => self.move_in_direction(Direction::Down),
                KeyCode::Char('k') | KeyCode::Up => self.move_in_direction(Direction::Up),
                KeyCode::Char('h') | KeyCode::Left => self.move_in_direction(Direction::Left),
                KeyCode::Char('l') | KeyCode::Right => self.move_in_direction(Direction::Right),
                KeyCode::Char('t') => {
                    self.tree_top_down = !self.tree_top_down;
                    true
                }
                KeyCode::Char('i') => {
                    self.show_interpretations = !self.show_interpretations;
                    true
                }
                KeyCode::Char('s') => {
                    self.effect_tableau_state.toggle_stretch();
                    true
                }
                _ => false,
            },
            KeyMouseEvent::Mouse(mouse) => {
                if mouse.kind != MouseEventKind::Down(MouseButton::Left) {
                    return false;
                }

                let clicked = self
                    .effect_tableau_state
                    .find_click_at(mouse.column, mouse.row);

                if let Some(sel) = clicked {
                    self.select_by_selection(sel);
                    return true;
                }
                false
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let content_area = render_error(frame, area, self.error.as_deref());

        if let Some(ref dag) = self.effect_dag {
            // Render monad stack info at the top
            let remaining = self.render_monad_stack(frame, content_area);

            // Render the DAG tree with integrated interpretation pane
            let widget = EffectTableauLayout::new(
                dag,
                self.tree_top_down,
                self.effect_tableau_state.stretch_enabled(),
                self.current_tree_selection(),
                self.show_interpretations,
            );
            frame.render_stateful_widget(widget, remaining, &mut self.effect_tableau_state);
            return;
        }

        frame.render_widget(
            Paragraph::new("No effect flow DAG available").style(Style::new().fg(Color::DarkGray)),
            content_area,
        );
    }
}

impl Mode for EffectFlowMode {
    type Model = EffectFlowModeInput;

    const NAME: &'static str = "Effect Flow";
    const KEYBINDINGS: &'static [(&'static str, &'static str)] = &[
        ("hjkl", "nav"),
        ("t", "flip"),
        ("i", "interp"),
        ("s", "stretch"),
    ];
    const SUPPORTED_FILTERS: &'static [FilterToggle] = &[];

    fn current_selection(&self) -> Option<Selection> {
        self.current_tree_selection()
    }

    fn legend_sections(&self) -> Vec<LegendSection> {
        vec![
            LegendSection::new(
                "Effect Kinds",
                Color::Cyan,
                vec![
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
                    LegendItem::new(EffectKind::EffectCatch.icon(), Theme::EFFECT_CATCH, "catch"),
                    LegendItem::new(EffectKind::Pure.icon(), Theme::EFFECT_PURE, "pure"),
                ],
            ),
            LegendSection::new(
                "Edge Kinds",
                Color::Magenta,
                vec![
                    LegendItem::new(
                        EdgeKind::DataFlow.icon(),
                        Theme::EDGE_DATA_FLOW,
                        "data flow",
                    ),
                    LegendItem::new(
                        EdgeKind::ControlFlow.icon(),
                        Theme::EDGE_CONTROL_FLOW,
                        "control",
                    ),
                    LegendItem::new(
                        EdgeKind::EffectFlow.icon(),
                        Theme::EDGE_EFFECT_FLOW,
                        "effect",
                    ),
                    LegendItem::new(
                        EdgeKind::ErrorPropagation.icon(),
                        Theme::EDGE_ERROR_PROPAGATION,
                        "error",
                    ),
                    LegendItem::new(
                        EdgeKind::EarlyReturn.icon(),
                        Theme::EDGE_EARLY_RETURN,
                        "early ret",
                    ),
                ],
            ),
        ]
    }
}
