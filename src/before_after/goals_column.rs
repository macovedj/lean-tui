use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    shared::{
        aligned_table::{AlignedRow, AlignedTable},
        diff_text::AnnotatedTextTreeExt,
        hypothesis_indices,
        theme::{Blue, Gray, Orange, Theme},
        HypothesisFilters, Region, Selection,
    },
};

/// State for the goals column widget (render artifacts only).
#[derive(Default)]
pub struct GoalsColumnState {
    click_regions: Vec<Region>,
}

impl GoalsColumnState {
    /// Get the click regions computed during the last render.
    pub fn click_regions(&self) -> &[Region] {
        &self.click_regions
    }
}

/// Widget for rendering a column showing proof state (hypotheses + goals).
pub struct GoalsColumn<'a> {
    title: &'a str,
    state: &'a TacticProofState,
    filters: HypothesisFilters,
    selection: Option<Selection>,
    is_current: bool,
    node_id: Option<NodeId>,
    active_goal_name: Option<&'a str>,
}

impl<'a> GoalsColumn<'a> {
    pub const fn new(
        title: &'a str,
        state: &'a TacticProofState,
        filters: HypothesisFilters,
        selection: Option<Selection>,
        is_current: bool,
        node_id: Option<NodeId>,
        active_goal_name: Option<&'a str>,
    ) -> Self {
        Self {
            title,
            state,
            filters,
            selection,
            is_current,
            node_id,
            active_goal_name,
        }
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

    fn is_hyp_focused(&self) -> bool {
        self.is_current && matches!(self.selection, Some(Selection::Hyp { .. }))
    }

    fn is_goal_focused(&self) -> bool {
        self.is_current && matches!(self.selection, Some(Selection::Goal { .. }))
    }
}

impl StatefulWidget for GoalsColumn<'_> {
    type State = GoalsColumnState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        // Render column title
        let title_style = if self.is_current {
            Style::new()
                .fg(Theme::CURRENT_NODE_BORDER)
                .add_modifier(Modifier::BOLD)
        } else {
            Theme::DIM
        };
        let title_line = Line::from(Span::styled(format!(" {} ", self.title), title_style));
        let title_area = Rect::new(area.x, area.y, area.width, 1);
        Paragraph::new(title_line)
            .alignment(Alignment::Center)
            .render(title_area, buf);

        let content_area = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(1),
        );

        if self.state.goals.is_empty() && self.state.hypotheses.is_empty() {
            render_empty_message(content_area, buf, self.is_current);
            return;
        }

        let visible_hyp_count =
            hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order)
                .filter(|&i| self.should_show_hypothesis(i))
                .count();

        // Calculate pane heights - assumptions get up to half, goals get the rest
        let hyp_pane_height =
            (visible_hyp_count + 2).min(content_area.height.saturating_sub(4) as usize / 2);
        let [hyp_pane_area, goal_pane_area] = Layout::vertical([
            Constraint::Length(hyp_pane_height as u16),
            Constraint::Fill(1),
        ])
        .areas(content_area);

        let node_id = self.is_current.then_some(self.node_id).flatten();
        let selection = self.is_current.then_some(self.selection).flatten();

        self.render_assumptions_pane(hyp_pane_area, buf, state, selection, node_id);
        self.render_goals_pane(goal_pane_area, buf, state, selection, node_id);
    }
}

impl GoalsColumn<'_> {
    fn render_assumptions_pane(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut GoalsColumnState,
        selection: Option<Selection>,
        node_id: Option<NodeId>,
    ) {
        let is_focused = self.is_hyp_focused();
        let border_color = if is_focused {
            Blue::BRIGHT
        } else if self.is_current {
            Blue::MUTED
        } else {
            Gray::DIM
        };
        let title = if is_focused {
            "▶ Assumptions "
        } else {
            " Assumptions "
        };
        let title_color = if self.is_current {
            Theme::TITLE_ASSUMPTION
        } else {
            Gray::DIM
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(title)
            .title_style(Style::new().fg(title_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.hypotheses.is_empty() {
            Paragraph::new("(none)")
                .style(Theme::DIM.add_modifier(Modifier::ITALIC))
                .render(inner, buf);
            return;
        }

        let visible_hyps: Vec<_> =
            hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order)
                .filter(|&i| self.should_show_hypothesis(i))
                .take(inner.height as usize)
                .collect();

        let rows: Vec<AlignedRow> = visible_hyps
            .iter()
            .map(|&hyp_idx| {
                let h = &self.state.hypotheses[hyp_idx];
                let is_selected =
                    matches!(selection, Some(Selection::Hyp { hyp_idx: sel, .. }) if sel == hyp_idx);

                let style = if is_selected {
                    Style::new().bg(Theme::SELECTION_BG)
                } else {
                    Style::default()
                };

                let (name_color, type_color) = if self.is_current {
                    (Theme::HYP_NAME, Theme::HYP_TYPE)
                } else {
                    (Gray::DIM, Gray::DIM)
                };

                AlignedRow::with_styled_type(
                    h.name.clone(),
                    style.fg(name_color),
                    ":",
                    Style::new().fg(Color::DarkGray),
                    h.type_.to_spans(style.fg(type_color)),
                )
            })
            .collect();

        let table = AlignedTable::new(rows);
        Widget::render(table, inner, buf);

        if let Some(nid) = node_id {
            register_hyp_regions(&mut state.click_regions, inner, nid, &visible_hyps);
        }
    }

    fn render_goals_pane(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut GoalsColumnState,
        selection: Option<Selection>,
        node_id: Option<NodeId>,
    ) {
        let is_focused = self.is_goal_focused();
        let border_color = if is_focused {
            Orange::BRIGHT
        } else if self.is_current {
            Orange::MUTED
        } else {
            Gray::DIM
        };
        let title = if is_focused { "▶ Goals " } else { " Goals " };
        let title_color = if self.is_current {
            Theme::TITLE_GOAL
        } else {
            Gray::DIM
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(title)
            .title_style(Style::new().fg(title_color));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.goals.is_empty() {
            let complete_color = if self.is_current {
                Theme::COMPLETED_GOAL_FG
            } else {
                Gray::DIM
            };
            Paragraph::new("✓ Complete")
                .style(Style::new().fg(complete_color).add_modifier(Modifier::BOLD))
                .render(inner, buf);
            return;
        }

        let visible_goals: Vec<_> = self
            .state
            .goals
            .iter()
            .enumerate()
            .take(inner.height as usize)
            .collect();

        let rows: Vec<AlignedRow> = visible_goals
            .iter()
            .map(|&(goal_idx, g)| {
                let is_selected =
                    matches!(selection, Some(Selection::Goal { goal_idx: sel, .. }) if sel == goal_idx);
                let is_active = self
                    .active_goal_name
                    .is_some_and(|name| g.username_str() == Some(name));

                let style = if is_selected {
                    Style::new().bg(Theme::SELECTION_BG)
                } else {
                    Style::default()
                };

                let target_style = if !self.is_current {
                    style.fg(Gray::DIM)
                } else if is_active {
                    style
                        .fg(Theme::CURRENT_NODE_BORDER)
                        .add_modifier(Modifier::BOLD)
                } else {
                    style.fg(Theme::GOAL_FG)
                };

                let name_text = g.username_str().unwrap_or_default().to_string();

                AlignedRow::with_styled_type(
                    name_text,
                    style,
                    "⊢",
                    Style::new().fg(Color::DarkGray),
                    g.type_.to_spans(target_style),
                )
            })
            .collect();

        let table = AlignedTable::new(rows);
        Widget::render(table, inner, buf);

        if let Some(nid) = node_id {
            register_goal_regions(&mut state.click_regions, inner, nid, &visible_goals);
        }
    }
}

fn register_hyp_regions(
    regions: &mut Vec<Region>,
    inner: Rect,
    node_id: NodeId,
    visible_hyps: &[usize],
) {
    for (row, &hyp_idx) in visible_hyps.iter().enumerate() {
        let row_y = inner.y + row as u16;
        if row_y >= inner.bottom() {
            break;
        }
        regions.push(Region::from_rect(
            Rect::new(inner.x, row_y, inner.width, 1),
            Selection::Hyp { node_id, hyp_idx },
        ));
    }
}

fn register_goal_regions<T>(
    regions: &mut Vec<Region>,
    inner: Rect,
    node_id: NodeId,
    visible_goals: &[(usize, T)],
) {
    for (row, (goal_idx, _)) in visible_goals.iter().enumerate() {
        let row_y = inner.y + row as u16;
        if row_y >= inner.bottom() {
            break;
        }
        regions.push(Region::from_rect(
            Rect::new(inner.x, row_y, inner.width, 1),
            Selection::Goal {
                node_id,
                goal_idx: *goal_idx,
            },
        ));
    }
}

fn render_empty_message(area: Rect, buf: &mut Buffer, is_current: bool) {
    let msg = if is_current { "No goals" } else { "No data" };
    Paragraph::new(msg).style(Theme::DIM).render(area, buf);
}
