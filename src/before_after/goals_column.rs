use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::{
    deduction_tree::{ProofContextHypothesis, ProofObligation, TacticProofState},
    protocol::NodeId,
    shared::{
        diff_text::AnnotatedTextTreeExt,
        hypothesis_indices,
        theme::{Blue, Orange, Theme},
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
            .alignment(ratatui::layout::Alignment::Center)
            .render(title_area, buf);

        let content_area = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));

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

        if self.state.hypotheses.is_empty() {
            Paragraph::new("(none)")
                .style(Theme::DIM.add_modifier(Modifier::ITALIC))
                .render(inner, buf);
            return;
        }

        let visible_hyps =
            hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order)
                .filter(|&i| self.should_show_hypothesis(i))
                .take(inner.height as usize);

        for (row, hyp_idx) in visible_hyps.enumerate() {
            let h = &self.state.hypotheses[hyp_idx];
            let is_selected =
                matches!(selection, Some(Selection::Hyp { hyp_idx: sel, .. }) if sel == hyp_idx);
            let line_area = Rect::new(inner.x, inner.y + row as u16, inner.width, 1);

            let line = render_hypothesis_line(h, is_selected);
            Paragraph::new(line).render(line_area, buf);

            if let Some(nid) = node_id {
                state.click_regions.push(Region::from_rect(
                    line_area,
                    Selection::Hyp {
                        node_id: nid,
                        hyp_idx,
                    },
                ));
            }
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
            Paragraph::new("✓ Complete")
                .style(
                    Style::new()
                        .fg(Theme::COMPLETED_GOAL_FG)
                        .add_modifier(Modifier::BOLD),
                )
                .render(inner, buf);
            return;
        }

        let visible_goals = self
            .state
            .goals
            .iter()
            .enumerate()
            .take(inner.height as usize);

        for (goal_idx, g) in visible_goals {
            let is_selected =
                matches!(selection, Some(Selection::Goal { goal_idx: sel, .. }) if sel == goal_idx);
            let is_active = self
                .active_goal_name
                .is_some_and(|name| g.username_str() == Some(name));
            let line_area = Rect::new(inner.x, inner.y + goal_idx as u16, inner.width, 1);

            let line = render_goal_line(g, is_selected, is_active);
            Paragraph::new(line).render(line_area, buf);

            if let Some(nid) = node_id {
                state.click_regions.push(Region::from_rect(
                    line_area,
                    Selection::Goal {
                        node_id: nid,
                        goal_idx,
                    },
                ));
            }
        }
    }
}

fn render_hypothesis_line(h: &ProofContextHypothesis, is_selected: bool) -> Line<'static> {
    let style = if is_selected {
        Style::new().bg(Theme::SELECTION_BG)
    } else {
        Style::default()
    };

    let mut spans = vec![
        Span::styled(h.name.clone(), style.fg(Theme::HYP_NAME)),
        Span::styled(" : ", style),
    ];
    spans.extend(h.type_.to_spans(style.fg(Theme::HYP_TYPE)));
    Line::from(spans)
}

fn render_goal_line(g: &ProofObligation, is_selected: bool, is_active: bool) -> Line<'static> {
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
        style.fg(Theme::GOAL_FG)
    };

    let prefix = g
        .username_str()
        .map_or_else(|| "⊢ ".to_string(), |name| format!("case {name} ⊢ "));

    let mut spans = vec![Span::styled(prefix, style)];
    spans.extend(g.type_.to_spans(target_style));
    Line::from(spans)
}

fn render_empty_message(area: Rect, buf: &mut Buffer, is_current: bool) {
    let msg = if is_current { "No goals" } else { "No data" };
    Paragraph::new(msg).style(Theme::DIM).render(area, buf);
}
