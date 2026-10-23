use std::iter::once;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Widget},
};

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

fn legend_lines(sections: &[LegendSection]) -> Vec<Line<'static>> {
    sections
        .iter()
        .flat_map(|section| {
            let title = Line::from(Span::styled(
                section.title.clone(),
                Style::new()
                    .fg(section.title_color)
                    .add_modifier(Modifier::BOLD),
            ));
            let items = section.items.iter().map(|item| {
                Line::from(vec![
                    Span::styled(item.icon.clone(), Style::new().fg(item.color)),
                    Span::raw(" "),
                    Span::raw(item.description.clone()),
                ])
            });
            once(title).chain(items)
        })
        .collect()
}

pub struct LegendPopup {
    pub sections: Vec<LegendSection>,
}

impl Widget for LegendPopup {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = legend_lines(&self.sections);
        let width = lines.iter().map(Line::width).max().unwrap_or_default() as u16 + 2;
        let height = (lines.len() as u16) + 2;

        let popup_area = Rect::new(1, area.height.saturating_sub(height + 1), width, height);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title(" Legend ")
            .border_style(Style::new().fg(Color::Reset));

        Paragraph::new(lines).block(block).render(popup_area, buf);
    }
}
