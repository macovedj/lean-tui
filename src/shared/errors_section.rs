use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{protocol::DiagnosticInfo, shared::theme::Red};

/// State for the errors section widget.
#[derive(Default)]
pub struct ErrorsSectionState {
    errors: Vec<DiagnosticInfo>,
}

impl ErrorsSectionState {
    pub fn update(&mut self, errors: Vec<DiagnosticInfo>) {
        self.errors = errors;
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Widget for rendering the errors section.
pub struct ErrorsSection;

impl StatefulWidget for ErrorsSection {
    type State = ErrorsSectionState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Red::MUTED))
            .title(Span::styled(
                " Errors ",
                Style::new().fg(Red::BRIGHT).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 2 || inner.height < 1 || state.errors.is_empty() {
            return;
        }

        let error_lines: Vec<Line> = state
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
