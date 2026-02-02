//! State node widget - renders a single proof state node in the semantic
//! tableau.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use super::given_pane::hyp_style_colors;
use crate::{
    protocol::{ProofDagTacticNode, TacticProofState},
    widgets::{tableau::truncate_str, theme::Theme, ClickRegion, Selection},
};

/// State for a single state node widget.
#[derive(Default)]
pub struct StateNodeState {
    pub click_regions: Vec<ClickRegion>,
}

/// A single proof state node widget.
pub struct StateNode<'a> {
    node: &'a ProofDagTacticNode,
    is_current: bool,
    selection: Option<Selection>,
    top_down: bool,
    override_state: Option<&'a TacticProofState>,
}

impl<'a> StateNode<'a> {
    pub const fn new(
        node: &'a ProofDagTacticNode,
        is_current: bool,
        selection: Option<Selection>,
        top_down: bool,
        override_state: Option<&'a TacticProofState>,
    ) -> Self {
        Self {
            node,
            is_current,
            selection,
            top_down,
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
            _ if self.node.has_unsolved_spawned_goals => Theme::INCOMPLETE_NODE_BORDER,
            (_, true, false) => Theme::INCOMPLETE_NODE_BORDER,
            _ => Theme::TACTIC_BORDER,
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

    fn build_hyps_line(&self) -> Option<Line<'static>> {
        if self.node.new_hypothesis_indices.is_empty() {
            return None;
        }

        let spans: Vec<Span> = self
            .node
            .new_hypothesis_indices
            .iter()
            .enumerate()
            .filter_map(|(i, &hyp_idx)| {
                let h = self.node.proof_state_after.hypotheses.get(hyp_idx as usize)?;
                let selected = matches!(
                    self.selection,
                    Some(Selection::Hyp { node_id, hyp_idx: hi }) if node_id == self.node.id && hi == hyp_idx as usize
                );
                let (fg, bg) = hyp_style_colors(h.is_proof_term);
                let style = Style::new()
                    .fg(fg)
                    .bg(bg)
                    .add_modifier(if selected { Modifier::UNDERLINED } else { Modifier::empty() });
                let text = format!(" {}: {} ", h.name, truncate_str(&h.type_.to_plain_text(), 20));
                let mut result = Vec::new();
                if i > 0 {
                    result.push(Span::raw(" "));
                }
                result.push(Span::styled(text, style));
                Some(result)
            })
            .flatten()
            .collect();

        Some(Line::from(spans))
    }

    fn build_goals_line(&self) -> Line<'static> {
        if self.is_effective_complete() {
            return Line::from(vec![Span::styled(
                "✓ Goal completed",
                Style::new()
                    .fg(Theme::COMPLETED_GOAL_FG)
                    .add_modifier(Modifier::BOLD),
            )]);
        }

        let mut spans: Vec<Span> = Vec::new();
        if self.node.is_leaf() {
            spans.push(Span::styled(
                "⋯ ",
                Style::new().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            ));
        }

        if let Some(state) = self.override_state {
            self.append_goal_spans_from_state(&mut spans, state);
        } else {
            self.append_goal_spans_from_node(&mut spans);
        }

        if spans.is_empty() {
            spans.push(Span::styled("⊢ ...", Style::new().fg(Theme::GOAL_FG)));
        }
        Line::from(spans)
    }

    fn append_goal_spans_from_state(
        &self,
        spans: &mut Vec<Span<'static>>,
        state: &TacticProofState,
    ) {
        for (goal_idx, g) in state.goals.iter().enumerate() {
            if goal_idx > 0 {
                spans.push(Span::styled(" │ ", Style::new().fg(Color::DarkGray)));
            }

            let selected = matches!(
                self.selection,
                Some(Selection::Goal { node_id, goal_idx: gi }) if node_id == self.node.id && gi == goal_idx
            );
            let underline = if selected {
                Modifier::UNDERLINED
            } else {
                Modifier::empty()
            };
            let goal_type = truncate_str(&g.type_.to_plain_text(), 35);

            if let Some(name) = g.username_str() {
                spans.push(Span::styled(
                    format!("{name}: "),
                    Style::new().fg(Color::Cyan).add_modifier(underline),
                ));
            }
            spans.push(Span::styled(
                format!("⊢ {goal_type}"),
                Style::new().fg(Theme::GOAL_FG).add_modifier(underline),
            ));
        }
    }

    fn append_goal_spans_from_node(&self, spans: &mut Vec<Span<'static>>) {
        for (goal_idx, g) in self.node.proof_state_after.goals.iter().enumerate() {
            if goal_idx > 0 {
                spans.push(Span::styled(" │ ", Style::new().fg(Color::DarkGray)));
            }

            let selected = matches!(
                self.selection,
                Some(Selection::Goal { node_id, goal_idx: gi }) if node_id == self.node.id && gi == goal_idx
            );
            let underline = if selected {
                Modifier::UNDERLINED
            } else {
                Modifier::empty()
            };
            let goal_type = truncate_str(&g.type_.to_plain_text(), 35);

            if let Some(name) = g.username_str() {
                spans.push(Span::styled(
                    format!("{name}: "),
                    Style::new().fg(Color::Cyan).add_modifier(underline),
                ));
            }
            spans.push(Span::styled(
                format!("⊢ {goal_type}"),
                Style::new().fg(Theme::GOAL_FG).add_modifier(underline),
            ));
        }
    }

    fn build_click_regions(&self, inner: Rect) -> Vec<ClickRegion> {
        let mut regions = Vec::new();
        let has_hyps = !self.node.new_hypothesis_indices.is_empty();

        let (hyps_y, goals_y) = if self.top_down {
            (inner.y, if has_hyps { inner.y + 1 } else { inner.y })
        } else {
            (if has_hyps { inner.y + 1 } else { inner.y }, inner.y)
        };

        let goal_count = self
            .override_state
            .map_or(self.node.proof_state_after.goals.len(), |s| s.goals.len());
        for goal_idx in 0..goal_count {
            regions.push(ClickRegion {
                area: Rect::new(inner.x, goals_y, inner.width, 1),
                selection: Selection::Goal {
                    node_id: self.node.id,
                    goal_idx,
                },
            });
        }

        if has_hyps {
            for &hyp_idx in &self.node.new_hypothesis_indices {
                regions.push(ClickRegion {
                    area: Rect::new(inner.x, hyps_y, inner.width, 1),
                    selection: Selection::Hyp {
                        node_id: self.node.id,
                        hyp_idx: hyp_idx as usize,
                    },
                });
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
                Color::White
            } else {
                Color::Gray
            })
            .add_modifier(if self.is_current {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });

        let arrow = if self.top_down { "▼" } else { "▲" };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(title, title_style))
            .title_bottom(Span::styled(format!(" {arrow} "), border_style));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        let hyps_line = self.build_hyps_line();
        let goals_line = self.build_goals_line();

        let lines: Vec<Line> = match (self.top_down, hyps_line) {
            (true, Some(h)) => vec![h, goals_line],
            (false, Some(h)) => vec![goals_line, h],
            (_, None) => vec![goals_line],
        };

        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .render(inner, buf);

        state.click_regions = self.build_click_regions(inner);
    }
}
