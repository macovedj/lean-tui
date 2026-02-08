use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
    Frame,
};

use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    shared::{
        aligned_table::{AlignedRow, AlignedTable},
        diff_text::RichTextExt,
        hypothesis_indices,
        theme::{Blue, Orange, Theme},
        HypothesisFilters, Region, Selection,
    },
};

pub struct OpenGoalList<'a> {
    state: &'a TacticProofState,
    selection: Option<Selection>,
    filters: HypothesisFilters,
    node_id: Option<NodeId>,
    active_goal_name: Option<&'a str>,
}

#[derive(Default)]
pub struct OpenGoalListState {
    click_regions: Vec<Region>,
}

impl<'a> OpenGoalList<'a> {
    pub const fn new(
        state: &'a TacticProofState,
        selection: Option<Selection>,
        filters: HypothesisFilters,
        node_id: Option<NodeId>,
        active_goal_name: Option<&'a str>,
    ) -> Self {
        Self {
            state,
            selection,
            filters,
            node_id,
            active_goal_name,
        }
    }

    pub fn render_to_frame(&self, frame: &mut Frame, area: Rect) -> Vec<Region> {
        let mut render_state = OpenGoalListState::default();
        frame.render_stateful_widget(
            OpenGoalList::new(
                self.state,
                self.selection,
                self.filters,
                self.node_id,
                self.active_goal_name,
            ),
            area,
            &mut render_state,
        );
        render_state.click_regions
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
        matches!(self.selection, Some(Selection::Hyp { .. }))
    }

    fn is_goal_focused(&self) -> bool {
        matches!(self.selection, Some(Selection::Goal { .. }))
    }
}

impl StatefulWidget for OpenGoalList<'_> {
    type State = OpenGoalListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let visible_hyp_count =
            hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order)
                .filter(|&i| self.should_show_hypothesis(i))
                .count();

        let hyp_pane_height = (visible_hyp_count + 2).min(area.height.saturating_sub(5) as usize);
        let [hyp_pane_area, goal_pane_area] = Layout::vertical([
            Constraint::Length(hyp_pane_height as u16),
            Constraint::Fill(1),
        ])
        .areas(area);

        self.render_hyp_pane(hyp_pane_area, buf, state);
        self.render_goal_pane(goal_pane_area, buf, state);
    }
}

impl OpenGoalList<'_> {
    fn render_hyp_pane(&self, area: Rect, buf: &mut Buffer, state: &mut OpenGoalListState) {
        let is_focused = self.is_hyp_focused();
        let border_color = if is_focused {
            Blue::BRIGHT
        } else {
            Blue::MUTED
        };
        let title = if is_focused {
            "▶ Assumptions "
        } else {
            " Assumptions "
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(title)
            .title_style(Style::new().fg(Theme::TITLE_ASSUMPTION));

        let inner = block.inner(area);
        block.render(area, buf);

        let visible_hyps: Vec<_> =
            hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order)
                .filter(|&i| self.should_show_hypothesis(i))
                .take(inner.height as usize)
                .collect();

        let rows: Vec<AlignedRow> = visible_hyps
            .iter()
            .map(|&hyp_idx| {
                let h = &self.state.hypotheses[hyp_idx];
                let is_selected = matches!(
                    self.selection,
                    Some(Selection::Hyp { hyp_idx: sel_idx, .. }) if sel_idx == hyp_idx
                );

                let style = if is_selected {
                    Style::new().bg(Theme::SELECTION_BG)
                } else {
                    Style::default()
                };

                AlignedRow::with_styled_type(
                    h.name.clone(),
                    style.fg(Theme::HYP_NAME),
                    ":",
                    Style::new().fg(Color::DarkGray),
                    h.type_.to_spans(style),
                )
            })
            .collect();

        let table = AlignedTable::new(rows);
        Widget::render(table, inner, buf);

        if let Some(nid) = self.node_id {
            register_hyp_regions(&mut state.click_regions, inner, nid, &visible_hyps);
        }
    }

    fn render_goal_pane(&self, area: Rect, buf: &mut Buffer, state: &mut OpenGoalListState) {
        let is_focused = self.is_goal_focused();
        let border_color = if is_focused {
            Orange::BRIGHT
        } else {
            Orange::MUTED
        };
        let title = if is_focused { "▶ Goals " } else { " Goals " };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(title)
            .title_style(Style::new().fg(Theme::TITLE_GOAL));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.goals.is_empty() {
            Paragraph::new("✓ All goals completed!")
                .style(
                    Style::new()
                        .fg(Theme::COMPLETED_GOAL_FG)
                        .add_modifier(Modifier::BOLD),
                )
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
                let is_selected = matches!(
                    self.selection,
                    Some(Selection::Goal { goal_idx: sel_idx, .. }) if sel_idx == goal_idx
                );
                let is_active = self
                    .active_goal_name
                    .is_some_and(|name| g.username_str() == Some(name));

                let style = if is_selected {
                    Style::new().bg(Theme::SELECTION_BG)
                } else {
                    Style::default()
                };

                let target_style = if is_active {
                    style
                        .fg(Theme::CURRENT_NODE_BORDER)
                        .add_modifier(Modifier::BOLD)
                } else {
                    style.fg(Theme::GOAL)
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

        if let Some(nid) = self.node_id {
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
