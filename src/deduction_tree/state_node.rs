use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Cell, Row, StatefulWidget, Table, Widget},
};

use super::{given_pane::hyp_style_colors, types::ProofDagTacticNode};
use crate::{
    deduction_tree::TacticProofState,
    shared::{
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

    fn max_name_width(&self) -> u16 {
        let hyp_max = self
            .node
            .new_hypothesis_indices
            .iter()
            .filter_map(|&idx| {
                self.node
                    .proof_state_after
                    .hypotheses
                    .get(idx as usize)
                    .map(|h| h.name.chars().count())
            })
            .max()
            .unwrap_or(0);

        let goals = self
            .override_state
            .map_or(&self.node.proof_state_after.goals, |s| &s.goals);

        let goal_max = goals
            .iter()
            .filter_map(|g| g.username_str().map(|s| s.chars().count()))
            .max()
            .unwrap_or(0);

        let is_leaf = self.node.is_leaf();
        let leaf_prefix = if is_leaf { 1 } else { 0 };

        (hyp_max.max(goal_max) + leaf_prefix) as u16
    }

    fn build_hyp_rows(&self) -> Vec<Row<'static>> {
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

                let name_style = Style::new().fg(fg).add_modifier(underline);
                let type_style = Style::new().fg(fg).add_modifier(underline);

                Some(Row::new(vec![
                    Cell::from(h.name.clone()).style(name_style),
                    Cell::from(":").style(Style::new().fg(Color::DarkGray)),
                    Cell::from(h.type_.to_plain_text()).style(type_style),
                ]))
            })
            .collect()
    }

    fn build_goal_rows(&self) -> Vec<Row<'static>> {
        if self.is_effective_complete() {
            return vec![Row::new(vec![
                Cell::from(""),
                Cell::from("✓").style(
                    Style::new()
                        .fg(Theme::COMPLETED_GOAL_FG)
                        .add_modifier(Modifier::BOLD),
                ),
                Cell::from("Goal completed").style(
                    Style::new()
                        .fg(Theme::COMPLETED_GOAL_FG)
                        .add_modifier(Modifier::BOLD),
                ),
            ])];
        }

        let goals = self
            .override_state
            .map_or(&self.node.proof_state_after.goals, |s| &s.goals);

        if goals.is_empty() {
            return vec![Row::new(vec![
                Cell::from(""),
                Cell::from("⊢").style(Style::new().fg(Color::DarkGray)),
                Cell::from("...").style(Style::new().fg(Theme::GOAL_FG)),
            ])];
        }

        let is_leaf = self.node.is_leaf();

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

                let name_text = match g.username_str() {
                    Some(name) => {
                        if is_leaf && goal_idx == 0 {
                            format!("⋯{name}")
                        } else {
                            name.to_string()
                        }
                    }
                    None => {
                        if is_leaf && goal_idx == 0 {
                            "⋯".to_string()
                        } else {
                            String::new()
                        }
                    }
                };

                let name_style = if g.username_str().is_some() {
                    Style::new().fg(Color::Cyan).add_modifier(underline)
                } else if is_leaf && goal_idx == 0 {
                    Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::new()
                };

                let type_style = Style::new().fg(Theme::GOAL_FG).add_modifier(underline);

                Row::new(vec![
                    Cell::from(name_text).style(name_style),
                    Cell::from("⊢").style(Style::new().fg(Color::DarkGray)),
                    Cell::from(g.type_.to_plain_text()).style(type_style),
                ])
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

        let mut rows: Vec<Row> = Vec::new();
        rows.extend(self.build_hyp_rows());
        rows.extend(self.build_goal_rows());

        let name_width = self.max_name_width().max(1);

        let table = Table::new(
            rows,
            [
                Constraint::Length(name_width),
                Constraint::Length(1),
                Constraint::Fill(1),
            ],
        );

        Widget::render(table, inner, buf);

        state.click_regions = self.build_click_regions(inner);
    }
}
