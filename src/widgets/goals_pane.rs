use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{
    deduction_tree::TacticProofState,
    message::{selection_click_target, ClickTarget},
    protocol::NodeId,
    selection::Selection,
    theme::{Gray, Orange, Theme},
    widgets::{
        aligned_table::{AlignedRow, AlignedTable},
        diff_text::RichTextExt,
    },
};

pub struct GoalsPane<'a> {
    state: &'a TacticProofState,
    selection: Option<Selection>,
    is_focused: bool,
    is_current: bool,
    node_id: Option<NodeId>,
    active_goal_name: Option<&'a str>,
}

impl<'a> GoalsPane<'a> {
    pub const fn new(
        state: &'a TacticProofState,
        selection: Option<Selection>,
        is_focused: bool,
        is_current: bool,
        node_id: Option<NodeId>,
        active_goal_name: Option<&'a str>,
    ) -> Self {
        Self {
            state,
            selection,
            is_focused,
            is_current,
            node_id,
            active_goal_name,
        }
    }
}

impl GoalsPane<'_> {
    pub fn draw(self, area: Rect, buf: &mut Buffer) -> Vec<ClickTarget> {
        let border_color = if self.is_focused {
            Orange::BRIGHT
        } else if self.is_current {
            Orange::MUTED
        } else {
            Gray::DIM
        };

        let title = if self.is_focused {
            "▶ Goals "
        } else {
            " Goals "
        };

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
            return Vec::new();
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
                    Some(Selection::Goal { goal_idx: sel, .. }) if sel == goal_idx
                );
                let is_active = self
                    .active_goal_name
                    .is_some_and(|name| g.username_str() == Some(name));

                let base_style = if is_selected {
                    Style::new().bg(Theme::SELECTION_BG)
                } else {
                    Style::default()
                };

                let name_style = if !self.is_current {
                    base_style.fg(Gray::DIM)
                } else if is_active {
                    base_style
                        .fg(Theme::CURRENT_NODE_BORDER)
                        .add_modifier(Modifier::BOLD)
                } else {
                    base_style.fg(Theme::GOAL)
                };

                let name_text = g.username_str().unwrap_or_default().to_string();

                AlignedRow::with_styled_type(
                    name_text,
                    name_style,
                    "⊢",
                    Style::new().fg(Gray::DIM),
                    g.type_.to_spans(base_style),
                )
            })
            .collect();

        let table = AlignedTable::new(rows);
        Widget::render(table, inner, buf);

        let Some(node_id) = self.node_id else {
            return Vec::new();
        };
        visible_goals
            .iter()
            .zip(inner.rows())
            .map(|(&(goal_idx, _), row_area)| {
                selection_click_target(row_area, Selection::Goal { node_id, goal_idx })
            })
            .collect()
    }
}
