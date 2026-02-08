use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use super::{
    assumption_pane::{AssumptionPane, AssumptionPaneState},
    goals_pane::{GoalsPane, GoalsPaneState},
};
use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    shared::{hypothesis_indices, theme::Theme, HypothesisFilters, Region, Selection},
};

pub struct CurrentPaneState {
    assumption_state: AssumptionPaneState,
    goals_state: GoalsPaneState,
}

impl Default for CurrentPaneState {
    fn default() -> Self {
        Self {
            assumption_state: AssumptionPaneState::default(),
            goals_state: GoalsPaneState::default(),
        }
    }
}

impl CurrentPaneState {
    pub fn click_regions(&self) -> impl Iterator<Item = &Region> {
        self.assumption_state
            .click_regions()
            .iter()
            .chain(self.goals_state.click_regions().iter())
    }
}

pub struct CurrentPane<'a> {
    state: &'a TacticProofState,
    filters: HypothesisFilters,
    selection: Option<Selection>,
    node_id: Option<NodeId>,
    active_goal_name: Option<&'a str>,
}

impl<'a> CurrentPane<'a> {
    pub const fn new(
        state: &'a TacticProofState,
        filters: HypothesisFilters,
        selection: Option<Selection>,
        node_id: Option<NodeId>,
        active_goal_name: Option<&'a str>,
    ) -> Self {
        Self {
            state,
            filters,
            selection,
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
        matches!(self.selection, Some(Selection::Hyp { .. }))
    }

    fn is_goal_focused(&self) -> bool {
        matches!(self.selection, Some(Selection::Goal { .. }))
    }
}

impl StatefulWidget for CurrentPane<'_> {
    type State = CurrentPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let title_style = Style::new()
            .fg(Theme::CURRENT_NODE_BORDER)
            .add_modifier(Modifier::BOLD);
        let title_line = Line::from(Span::styled(" Current ", title_style));
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

        let visible_hyp_count =
            hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order)
                .filter(|&i| self.should_show_hypothesis(i))
                .count();

        let hyp_pane_height =
            (visible_hyp_count + 2).min(content_area.height.saturating_sub(4) as usize / 2);

        let [hyp_area, goal_area] = Layout::vertical([
            Constraint::Length(hyp_pane_height as u16),
            Constraint::Fill(1),
        ])
        .areas(content_area);

        AssumptionPane::new(
            self.state,
            self.filters,
            self.selection,
            self.is_hyp_focused(),
            true,
            self.node_id,
        )
        .render(hyp_area, buf, &mut state.assumption_state);

        GoalsPane::new(
            self.state,
            self.selection,
            self.is_goal_focused(),
            true,
            self.node_id,
            self.active_goal_name,
        )
        .render(goal_area, buf, &mut state.goals_state);
    }
}
