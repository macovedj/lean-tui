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
    pub icon: String,
    pub color: Color,
    pub description: String,
}

impl LegendItem {
    pub fn new(icon: impl Into<String>, color: Color, description: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            color,
            description: description.into(),
        }
    }
}

/// A section in the legend with a title and items.
#[derive(Debug, Clone)]
pub struct LegendSection {
    pub title: String,
    pub title_color: Color,
    pub items: Vec<LegendItem>,
}

impl LegendSection {
    pub fn new(title: impl Into<String>, title_color: Color, items: Vec<LegendItem>) -> Self {
        Self {
            title: title.into(),
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

        for section in self.sections {
            lines.push(Line::from(Span::styled(
                section.title.clone(),
                Style::new()
                    .fg(section.title_color)
                    .add_modifier(Modifier::BOLD),
            )));

            for item in &section.items {
                lines.push(Line::from(vec![
                    Span::styled(item.icon.clone(), Style::new().fg(item.color)),
                    Span::raw(" "),
                    Span::raw(item.description.clone()),
                ]));
            }
        }

        lines
    }

    pub fn width(&self) -> u16 {
        let lines = self.build_lines();
        lines
            .iter()
            .map(ratatui::prelude::Line::width)
            .max()
            .unwrap_or_default() as u16
            + 2
    }

    pub fn height(&self) -> u16 {
        let lines = self.build_lines();
        (lines.len() as u16) + 2
    }
}

impl StatefulWidget for LegendWidget<'_> {
    type State = Legend;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

        let lines = self.build_lines();
        let width = self.width();
        let height = self.height();

        let x = 1;
        let y = area.height.saturating_sub(height + 1);
        let popup_area = Rect::new(x, y, width, height);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title(" Legend ")
            .border_style(Style::new().fg(Color::Reset));

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
