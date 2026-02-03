//! Theorem pane - displays the theorem being proved in the semantic tableau.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::shared::{theme::Theme, ClickRegion, Selection};

/// State for the theorem pane.
#[derive(Default)]
pub struct TheoremPaneState {
    pub click_regions: Vec<ClickRegion>,
}

impl TheoremPaneState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(Rect::new(x, y, 1, 1)))
            .map(|r| r.selection)
    }
}

/// Theorem pane widget - displays the theorem conclusion.
pub struct TheoremPane<'a> {
    goal: &'a str,
    selection: Option<Selection>,
}

impl<'a> TheoremPane<'a> {
    pub const fn new(goal: &'a str, selection: Option<Selection>) -> Self {
        Self { goal, selection }
    }
}

impl StatefulWidget for TheoremPane<'_> {
    type State = TheoremPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let is_selected = matches!(self.selection, Some(Selection::Theorem));

        state.click_regions.push(ClickRegion {
            area,
            selection: Selection::Theorem,
        });

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::Magenta))
            .title(Span::styled(
                " THEOREM ",
                Style::new().fg(Color::Magenta).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        let style = Style::new()
            .fg(Theme::GOAL_FG)
            .add_modifier(if is_selected {
                Modifier::UNDERLINED
            } else {
                Modifier::empty()
            });

        Paragraph::new(Line::from(vec![Span::styled(
            format!("⊢ {}", self.goal),
            style,
        )]))
        .wrap(Wrap { trim: true })
        .render(inner, buf);
    }
}
