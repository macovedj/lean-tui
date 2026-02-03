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
        types::{
            EdgeKind, EffectFlowDag, EffectFlowDagExt, EffectFlowEdgeExt, EffectFlowNodeExt,
            EffectKind,
        },
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

/// Effect Flow display mode - monadic effect visualization.
///
/// Visualizes effect flow using forward/backward function decomposition,
/// inspired by Aeneas. Shows state reads/writes, effect binds, and
/// error propagation.
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

    /// Render effect interpretation info for the selected node.
    fn render_interpretations(&self, frame: &mut Frame, area: Rect) -> Rect {
        if !self.show_interpretations {
            return area;
        }

        let Some(dag) = &self.effect_dag else {
            return area;
        };

        // Get the selected node
        let selected_node_id = match self.current_tree_selection() {
            Some(Selection::Goal { node_id, .. }) => Some(node_id),
            _ => dag.current_node_id,
        };

        let Some(node_id) = selected_node_id else {
            return area;
        };

        let Some(node) = dag.get(node_id) else {
            return area;
        };

        let mut lines = Vec::new();

        // Effect kind header with icon
        let effect_kind = node.effect_kind();
        lines.push(Line::from(vec![
            Span::styled(
                format!("{} ", effect_kind.icon()),
                Style::new().fg(effect_kind.color()),
            ),
            Span::styled(
                effect_kind.description(),
                Style::new()
                    .fg(effect_kind.color())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" ({})", node.content.to_plain_text()),
                Style::new().fg(Color::DarkGray),
            ),
        ]));

        // Forward/backward interpretations
        if let Some(forward) = node.forward() {
            lines.push(Line::from(vec![
                Span::styled("  fwd: ", Style::new().fg(Color::Green)),
                Span::styled(forward, Style::new().fg(Color::White)),
            ]));
        }

        if let Some(backward) = node.backward() {
            lines.push(Line::from(vec![
                Span::styled("  bwd: ", Style::new().fg(Color::Magenta)),
                Span::styled(backward, Style::new().fg(Color::White)),
            ]));
        }

        // Bindings read/written
        let bindings_read = node.bindings_read();
        if !bindings_read.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  reads: ", Style::new().fg(Color::Cyan)),
                Span::styled(bindings_read.join(", "), Style::new().fg(Color::White)),
            ]));
        }

        let bindings_written = node.bindings_written();
        if !bindings_written.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  writes: ", Style::new().fg(Color::Yellow)),
                Span::styled(bindings_written.join(", "), Style::new().fg(Color::White)),
            ]));
        }

        // Outgoing edges with their kinds
        if !node.edges.is_empty() {
            lines.push(render_edge_line(&node.edges));
        }

        if lines.is_empty() {
            return area;
        }

        let height = lines.len() as u16;
        let info_area = Rect::new(area.x, area.y, area.width, height.min(area.height));
        frame.render_widget(Paragraph::new(lines), info_area);

        // Return remaining area
        if area.height > height {
            Rect::new(area.x, area.y + height, area.width, area.height - height)
        } else {
            Rect::new(area.x, area.y + area.height, area.width, 0)
        }
    }
}

/// Render a line showing edge information.
fn render_edge_line(edges: &[crate::protocol::GraphEdge]) -> Line<'static> {
    let edge_spans: Vec<Span> = edges
        .iter()
        .enumerate()
        .flat_map(|(i, edge)| {
            let edge_kind = edge.edge_kind();
            let label = edge.label.as_deref().unwrap_or("");
            let target_text = if label.is_empty() {
                format!("→{}", edge.target)
            } else {
                format!("{}→{}", label, edge.target)
            };
            let mut spans = vec![
                Span::styled(
                    edge_kind.icon().to_string(),
                    Style::new().fg(edge_kind.color()),
                ),
                Span::styled(target_text, Style::new().fg(Color::DarkGray)),
            ];
            if i < edges.len() - 1 {
                spans.push(Span::raw(" "));
            }
            spans
        })
        .collect();

    let mut line_spans = vec![Span::styled("  edges: ", Style::new().fg(Color::Gray))];
    line_spans.extend(edge_spans);
    Line::from(line_spans)
}

impl InteractiveComponent for EffectFlowMode {
    type Input = EffectFlowModeInput;
    type Event = KeyMouseEvent;

    fn update(&mut self, input: Self::Input) {
        let old_count = self.effect_dag.as_ref().map_or(0, EffectFlowDag::len);
        let new_count = input.effect_dag.as_ref().map_or(0, EffectFlowDag::len);

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
            // Render monad stack info
            let remaining = self.render_monad_stack(frame, content_area);

            // Render forward/backward interpretations
            let remaining = self.render_interpretations(frame, remaining);

            // Render the DAG tree (use inner GenericDag for layout)
            let widget = EffectTableauLayout::new(
                dag.as_generic(),
                self.tree_top_down,
                self.effect_tableau_state.stretch_enabled(),
                self.current_tree_selection(),
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
