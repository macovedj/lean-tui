use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
    Frame,
};

use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    shared::{
        diff_text::AnnotatedTextTreeExt,
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

        if self.state.goals.is_empty() && self.state.hypotheses.is_empty() {
            Paragraph::new("No goals")
                .style(Theme::DIM)
                .render(area, buf);
            return;
        }

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
            .title_style(Style::new().fg(Theme::TITLE_HYPOTHESIS));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.hypotheses.is_empty() {
            Paragraph::new("(no assumptions)")
                .style(Theme::DIM.add_modifier(Modifier::ITALIC))
                .render(inner, buf);
            return;
        }

        let mut y = inner.y;
        for hyp_idx in hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order) {
            if !self.should_show_hypothesis(hyp_idx) {
                continue;
            }
            if y >= inner.bottom() {
                break;
            }

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

            let mut spans = vec![
                Span::styled(&h.name, style.fg(Theme::HYP_NAME)),
                Span::styled(" : ", style),
            ];
            spans.extend(h.type_.to_spans(style.fg(Theme::HYP_TYPE)));
            let line = Line::from(spans);

            let line_area = Rect::new(inner.x, y, inner.width, 1);
            Paragraph::new(line).render(line_area, buf);

            if let Some(nid) = self.node_id {
                state.click_regions.push(Region::from_rect(
                    line_area,
                    Selection::Hyp {
                        node_id: nid,
                        hyp_idx,
                    },
                ));
            }

            y += 1;
        }
    }

    fn render_goal_pane(&self, area: Rect, buf: &mut Buffer, state: &mut OpenGoalListState) {
        let is_focused = self.is_goal_focused();
        let border_color = if is_focused {
            Orange::BRIGHT
        } else {
            Orange::MUTED
        };
        let title = if is_focused {
            "▶ Goals "
        } else {
            " Goals "
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(title)
            .title_style(Style::new().fg(Theme::TITLE_GOAL));

        let inner = block.inner(area);
        block.render(area, buf);

        if self.state.goals.is_empty() {
            Paragraph::new("✓ All goals completed!")
                .style(Style::new().fg(Theme::COMPLETED_GOAL_FG).add_modifier(Modifier::BOLD))
                .render(inner, buf);
            return;
        }

        let mut y = inner.y;
        for (goal_idx, g) in self.state.goals.iter().enumerate() {
            if y >= inner.bottom() {
                break;
            }

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
                style.fg(Theme::GOAL_TYPE)
            };

            let prefix = g
                .username_str()
                .map_or_else(|| "⊢ ".to_string(), |name| format!("case {name} ⊢ "));
            let mut spans = vec![Span::styled(prefix, style)];
            spans.extend(g.type_.to_spans(target_style));
            let line = Line::from(spans);

            let line_area = Rect::new(inner.x, y, inner.width, 1);
            Paragraph::new(line).render(line_area, buf);

            if let Some(nid) = self.node_id {
                state.click_regions.push(Region::from_rect(
                    line_area,
                    Selection::Goal {
                        node_id: nid,
                        goal_idx,
                    },
                ));
            }

            y += 1;
        }
    }
}
