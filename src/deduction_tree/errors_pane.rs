use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{protocol::DiagnosticInfo, selection::Region, theme::Red};

/// State for the errors pane.
#[derive(Default)]
pub struct ErrorsPaneState {
    pub click_regions: Vec<Region>,
}

/// Errors pane widget - displays diagnostic errors.
pub struct ErrorsPane<'a> {
    errors: &'a [DiagnosticInfo],
}

impl<'a> ErrorsPane<'a> {
    pub const fn new(errors: &'a [DiagnosticInfo]) -> Self {
        Self { errors }
    }
}

impl StatefulWidget for ErrorsPane<'_> {
    type State = ErrorsPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Red::BRIGHT))
            .title(Span::styled(
                " Errors ",
                Style::new().fg(Red::BRIGHT).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 2 || inner.height < 1 || self.errors.is_empty() {
            return;
        }

        let error_lines: Vec<Line> = self
            .errors
            .iter()
            .flat_map(|e| {
                e.message.lines().enumerate().map(|(i, line)| {
                    let prefix = if i == 0 { "✗ " } else { "  " };
                    Line::from(Span::styled(
                        format!("{prefix}{line}"),
                        Style::new().fg(Red::BRIGHT),
                    ))
                })
            })
            .collect();

        Paragraph::new(error_lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}
