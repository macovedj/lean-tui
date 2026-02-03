//! Error banner widget with line wrapping.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

/// Widget for displaying error messages with line wrapping.
pub struct ErrorBanner<'a> {
    message: &'a str,
}

impl<'a> ErrorBanner<'a> {
    pub const fn new(message: &'a str) -> Self {
        Self { message }
    }

    /// Calculate the height needed to display the error message within the
    /// given width.
    pub fn required_height(&self, width: u16) -> u16 {
        if self.message.is_empty() || width == 0 {
            return 0;
        }

        // Account for border (2) and padding
        let content_width = width.saturating_sub(4) as usize;

        if content_width == 0 {
            return 3; // Minimum height with borders
        }

        let line_count = self
            .message
            .chars()
            .collect::<Vec<_>>()
            .chunks(content_width)
            .count();

        // Add 2 for top and bottom border
        (line_count as u16).max(1) + 2
    }
}

impl Widget for ErrorBanner<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if self.message.is_empty() || area.is_empty() {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Color::Red))
            .title(Span::styled(
                " Error ",
                Style::new().fg(Color::Red).add_modifier(Modifier::BOLD),
            ))
            .title_top(Line::from(vec![
                Span::styled("[y]", Style::new().fg(Color::DarkGray)),
                Span::styled(" copy ", Style::new().fg(Color::Gray)),
            ]).right_aligned());

        let text = Line::from(Span::styled(self.message, Style::new().fg(Color::Red)));

        Paragraph::new(text)
            .block(block)
            .wrap(Wrap { trim: false })
            .render(area, buf);
    }
}
