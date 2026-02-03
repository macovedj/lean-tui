//! Legend popup overlay for displaying color/icon meanings.

use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, StatefulWidget, Widget},
};

use super::{InteractiveStatefulWidget, KeyEvent};

/// A single legend entry with icon, color, and description.
#[derive(Debug, Clone)]
pub struct LegendItem {
    pub icon: &'static str,
    pub color: Color,
    pub description: &'static str,
}

impl LegendItem {
    pub const fn new(icon: &'static str, color: Color, description: &'static str) -> Self {
        Self {
            icon,
            color,
            description,
        }
    }
}

/// A section in the legend with a title and items.
#[derive(Debug, Clone)]
pub struct LegendSection {
    pub title: &'static str,
    pub title_color: Color,
    pub items: Vec<LegendItem>,
}

impl LegendSection {
    pub fn new(title: &'static str, title_color: Color, items: Vec<LegendItem>) -> Self {
        Self {
            title,
            title_color,
            items,
        }
    }
}

/// State for the legend popup.
#[derive(Default)]
pub struct Legend {
    visible: bool,
}

impl Legend {
    pub const fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub const fn is_visible(&self) -> bool {
        self.visible
    }
}

/// Widget for rendering the legend popup overlay.
pub struct LegendWidget<'a> {
    sections: &'a [LegendSection],
}

impl<'a> LegendWidget<'a> {
    pub const fn new(sections: &'a [LegendSection]) -> Self {
        Self { sections }
    }

    fn build_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let dim = Style::new().fg(Color::DarkGray);

        for (i, section) in self.sections.iter().enumerate() {
            if i > 0 {
                lines.push(Line::from(Span::styled("────────────────────", dim)));
            }

            lines.push(Line::from(Span::styled(
                section.title,
                Style::new()
                    .fg(section.title_color)
                    .add_modifier(Modifier::BOLD),
            )));

            for item in &section.items {
                lines.push(Line::from(vec![
                    Span::styled(item.icon, Style::new().fg(item.color)),
                    Span::raw(" "),
                    Span::raw(item.description),
                ]));
            }
        }

        lines
    }
}

impl StatefulWidget for LegendWidget<'_> {
    type State = Legend;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

        let lines = self.build_lines();
        let width = 22u16;
        let height = (lines.len() as u16) + 2;

        // Position in bottom left corner
        let x = 1;
        let y = area.height.saturating_sub(height + 1);
        let popup_area = Rect::new(x, y, width.min(area.width.saturating_sub(2)), height);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title(" Legend ")
            .border_style(Style::new().fg(Color::DarkGray));

        Paragraph::new(lines).block(block).render(popup_area, buf);
    }
}

impl InteractiveStatefulWidget for LegendWidget<'_> {
    type Input = ();
    type Event = KeyEvent;

    fn update_state(_state: &mut Self::State, _input: Self::Input) {}

    fn handle_event(state: &mut Self::State, event: Self::Event) -> bool {
        if !state.visible {
            return false;
        }

        match event.code {
            KeyCode::Esc | KeyCode::Char('L') => {
                state.visible = false;
                true
            }
            _ => false,
        }
    }
}
