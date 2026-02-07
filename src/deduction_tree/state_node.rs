use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use super::{given_pane::hyp_style_colors, types::ProofDagTacticNode};
use crate::{
    deduction_tree::TacticProofState,
    shared::{
        aligned_table::{AlignedRow, AlignedTable},
        theme::Theme,
        Region, Selection,
    },
};

/// State for a single state node widget.
#[derive(Default)]
pub struct StateNodeState {
    pub click_regions: Vec<Region>,
}

/// A single proof state node widget.
pub struct StateNode<'a> {
    node: &'a ProofDagTacticNode,
    is_current: bool,
    selection: Option<Selection>,
    override_state: Option<&'a TacticProofState>,
}

impl<'a> StateNode<'a> {
    pub const fn new(
        node: &'a ProofDagTacticNode,
        is_current: bool,
        selection: Option<Selection>,
        override_state: Option<&'a TacticProofState>,
    ) -> Self {
        Self {
            node,
            is_current,
            selection,
            override_state,
        }
    }

    fn is_effective_complete(&self) -> bool {
        self.override_state
            .map_or_else(|| self.node.is_complete(), |s| s.goals.is_empty())
    }

    fn border_color(&self) -> Color {
        let dominated_vars = (
            self.is_current,
            self.node.is_leaf(),
            self.is_effective_complete(),
        );
        match dominated_vars {
            (true, _, _) => Theme::CURRENT_NODE_BORDER,
            (_, true, true) => Theme::COMPLETED_NODE_BORDER,
            (_, true, false) => Theme::INCOMPLETE_NODE_BORDER,
            (_, false, _) => Theme::TACTIC_BORDER,
        }
    }

    fn build_title(&self) -> String {
        if self.node.children.len() > 1 {
            format!(
                " {} [{}→] ",
                self.node.tactic.text,
                self.node.children.len()
            )
        } else {
            format!(" {} ", self.node.tactic.text)
        }
    }

    fn build_hyp_rows(&self) -> Vec<AlignedRow> {
        self.node
            .new_hypothesis_indices
            .iter()
            .filter_map(|&hyp_idx| {
                let h = self.node.proof_state_after.hypotheses.get(hyp_idx as usize)?;
                let selected = matches!(
                    self.selection,
                    Some(Selection::Hyp { node_id, hyp_idx: hi }) if node_id == self.node.id && hi == hyp_idx as usize
                );
                let (fg, _bg) = hyp_style_colors(h.is_proof_term);
                let underline = if selected {
                    Modifier::UNDERLINED
                } else {
                    Modifier::empty()
                };

                let style = Style::new().fg(fg).add_modifier(underline);

                Some(AlignedRow::new(
                    h.name.clone(),
                    style,
                    ":",
                    Style::new().fg(Color::DarkGray),
                    h.type_.to_plain_text(),
                    style,
                ))
            })
            .collect()
    }

    fn build_goal_rows(&self) -> Vec<AlignedRow> {
        if self.is_effective_complete() {
            let completed_style = Style::new()
                .fg(Theme::COMPLETED_GOAL_FG)
                .add_modifier(Modifier::BOLD);
            return vec![AlignedRow::new(
                "",
                Style::default(),
                "✓",
                completed_style,
                "Goal completed",
                completed_style,
            )];
        }

        let goals = self
            .override_state
            .map_or(&self.node.proof_state_after.goals, |s| &s.goals);

        if goals.is_empty() {
            return vec![AlignedRow::new(
                "",
                Style::default(),
                "⊢",
                Style::new().fg(Color::DarkGray),
                "...",
                Style::new().fg(Theme::GOAL_FG),
            )];
        }

        goals
            .iter()
            .enumerate()
            .map(|(goal_idx, g)| {
                let selected = matches!(
                    self.selection,
                    Some(Selection::Goal { node_id, goal_idx: gi }) if node_id == self.node.id && gi == goal_idx
                );
                let underline = if selected {
                    Modifier::UNDERLINED
                } else {
                    Modifier::empty()
                };

                let name_text = g.username_str().unwrap_or_default().to_string();

                let name_style = if g.username_str().is_some() {
                    Style::new().fg(Theme::GOAL_FG).add_modifier(underline)
                } else {
                    Style::new()
                };

                let type_style = Style::new().fg(Theme::GOAL_FG).add_modifier(underline);

                AlignedRow::new(
                    name_text,
                    name_style,
                    "⊢",
                    Style::new().fg(Color::DarkGray),
                    g.type_.to_plain_text(),
                    type_style,
                )
            })
            .collect()
    }

    fn build_click_regions(&self, inner: Rect) -> Vec<Region> {
        let mut regions = Vec::new();
        let hyp_count = self.node.new_hypothesis_indices.len();

        let goal_count = self
            .override_state
            .map_or(self.node.proof_state_after.goals.len(), |s| s.goals.len());

        let mut current_y = inner.y;

        for (i, &hyp_idx) in self.node.new_hypothesis_indices.iter().enumerate() {
            let row_y = current_y + i as u16;
            if row_y < inner.y + inner.height {
                regions.push(Region::from_rect(
                    Rect::new(inner.x, row_y, inner.width, 1),
                    Selection::Hyp {
                        node_id: self.node.id,
                        hyp_idx: hyp_idx as usize,
                    },
                ));
            }
        }

        current_y += hyp_count as u16;

        for goal_idx in 0..goal_count {
            let row_y = current_y + goal_idx as u16;
            if row_y < inner.y + inner.height {
                regions.push(Region::from_rect(
                    Rect::new(inner.x, row_y, inner.width, 1),
                    Selection::Goal {
                        node_id: self.node.id,
                        goal_idx,
                    },
                ));
            }
        }

        regions
    }
}

impl StatefulWidget for StateNode<'_> {
    type State = StateNodeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let border_color = self.border_color();
        let border_style = Style::new()
            .fg(border_color)
            .add_modifier(if self.is_current {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });

        let title = self.build_title();
        let title_style = Style::new()
            .fg(if self.is_current {
                Theme::CURRENT_NODE_BORDER
            } else {
                Color::Gray
            })
            .add_modifier(if self.is_current {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, title_style));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        let mut rows: Vec<AlignedRow> = Vec::new();
        rows.extend(self.build_hyp_rows());
        rows.extend(self.build_goal_rows());

        let table = AlignedTable::new(rows);
        Widget::render(table, inner, buf);

        state.click_regions = self.build_click_regions(inner);
    }
}
