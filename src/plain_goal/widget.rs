use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget},
    Frame,
};

use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    shared::{
        diff_text::AnnotatedTextTreeExt, hypothesis_indices, theme::Theme, Region,
        HypothesisFilters, Selection,
    },
};

pub struct OpenGoalList<'a> {
    state: &'a TacticProofState,
    selection: Option<Selection>,
    filters: HypothesisFilters,
    /// Node ID for creating click region selections.
    node_id: Option<NodeId>,
    /// Name of the goal the cursor's tactic is working on.
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

    /// Render using Frame (convenience method for non-stateful usage).
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

        let hyp_height = visible_hyp_count.min(area.height.saturating_sub(3) as usize / 2);
        let constraints = vec![
            Constraint::Length(hyp_height as u16),
            Constraint::Length(1), // divider
            Constraint::Fill(1),   // goals
        ];
        let [hyp_area, div_area, goal_area] = Layout::vertical(constraints).areas(area);

        let mut y = hyp_area.y;
        for hyp_idx in hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order) {
            if !self.should_show_hypothesis(hyp_idx) {
                continue;
            }
            if y >= hyp_area.bottom() {
                break;
            }

            let h = &self.state.hypotheses[hyp_idx];
            let is_selected = matches!(self.selection, Some(Selection::Hyp { hyp_idx: sel_idx, .. }) if sel_idx == hyp_idx);

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

            let line_area = Rect::new(hyp_area.x, y, hyp_area.width, 1);
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

        let divider = "─".repeat(div_area.width as usize);
        Paragraph::new(divider)
            .style(Theme::DIM)
            .render(div_area, buf);

        y = goal_area.y;
        for (goal_idx, g) in self.state.goals.iter().enumerate() {
            if y >= goal_area.bottom() {
                break;
            }

            let is_selected = matches!(self.selection, Some(Selection::Goal { goal_idx: sel_idx, .. }) if sel_idx == goal_idx);
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

            let line_area = Rect::new(goal_area.x, y, goal_area.width, 1);
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
