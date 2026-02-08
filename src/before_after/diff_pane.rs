use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    shared::{
        hypothesis_indices, theme::Gray, theme::Theme, AssumptionPane, AssumptionPaneState,
        GoalsPane, GoalsPaneState, HypothesisFilters, Region, Selection,
    },
};

#[derive(Default)]
pub struct DiffPaneState {
    assumption_state: AssumptionPaneState,
    goals_state: GoalsPaneState,
}

impl DiffPaneState {
    pub fn click_regions(&self) -> impl Iterator<Item = &Region> {
        self.assumption_state
            .click_regions()
            .iter()
            .chain(self.goals_state.click_regions().iter())
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum DiffPaneKind {
    Before,
    Current,
    After,
}

impl DiffPaneKind {
    const fn title(self) -> &'static str {
        match self {
            Self::Before => " Before ",
            Self::Current => " Current ",
            Self::After => " After ",
        }
    }

    fn title_style(self) -> Style {
        match self {
            Self::Before | Self::After => Style::new().fg(Gray::DIM).add_modifier(Modifier::BOLD),
            Self::Current => Style::new()
                .fg(Theme::CURRENT_NODE_BORDER)
                .add_modifier(Modifier::BOLD),
        }
    }

    const fn is_current(self) -> bool {
        matches!(self, Self::Current)
    }
}

fn should_show_hypothesis(
    state: &TacticProofState,
    filters: HypothesisFilters,
    idx: usize,
) -> bool {
    let Some(h) = state.hypotheses.get(idx) else {
        return false;
    };
    if filters.hide_instances && h.is_typeclass_instance {
        return false;
    }
    if filters.hide_inaccessible && h.is_proof_term {
        return false;
    }
    true
}

pub struct DiffPane<'a> {
    kind: DiffPaneKind,
    state: &'a TacticProofState,
    filters: HypothesisFilters,
    selection: Option<Selection>,
    node_id: Option<NodeId>,
    active_goal_name: Option<&'a str>,
}

impl<'a> DiffPane<'a> {
    pub const fn new(
        kind: DiffPaneKind,
        state: &'a TacticProofState,
        filters: HypothesisFilters,
        selection: Option<Selection>,
        node_id: Option<NodeId>,
        active_goal_name: Option<&'a str>,
    ) -> Self {
        Self {
            kind,
            state,
            filters,
            selection,
            node_id,
            active_goal_name,
        }
    }

    fn is_hyp_focused(&self) -> bool {
        self.kind.is_current() && matches!(self.selection, Some(Selection::Hyp { .. }))
    }

    fn is_goal_focused(&self) -> bool {
        self.kind.is_current() && matches!(self.selection, Some(Selection::Goal { .. }))
    }
}

impl StatefulWidget for DiffPane<'_> {
    type State = DiffPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let title_line = Line::from(Span::styled(self.kind.title(), self.kind.title_style()));
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
                .filter(|&i| should_show_hypothesis(self.state, self.filters, i))
                .count();

        let hyp_pane_height =
            (visible_hyp_count + 2).min(content_area.height.saturating_sub(4) as usize / 2);

        let [hyp_area, goal_area] = Layout::vertical([
            Constraint::Length(hyp_pane_height as u16),
            Constraint::Fill(1),
        ])
        .areas(content_area);

        let is_current = self.kind.is_current();
        let selection = if is_current { self.selection } else { None };
        let node_id = if is_current { self.node_id } else { None };

        AssumptionPane::new(
            self.state,
            self.filters,
            selection,
            self.is_hyp_focused(),
            is_current,
            node_id,
        )
        .render(hyp_area, buf, &mut state.assumption_state);

        GoalsPane::new(
            self.state,
            selection,
            self.is_goal_focused(),
            is_current,
            node_id,
            self.active_goal_name,
        )
        .render(goal_area, buf, &mut state.goals_state);
    }
}
