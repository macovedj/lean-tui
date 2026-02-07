use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    protocol::DiagnosticInfo,
    shared::theme::Red,
};

/// State for the errors section widget.
#[derive(Default)]
pub struct ErrorsSectionState {
    errors: Vec<DiagnosticInfo>,
    /// Whether this pane is currently focused.
    is_focused: bool,
}

impl ErrorsSectionState {
    pub fn update(&mut self, errors: Vec<DiagnosticInfo>) {
        self.errors = errors;
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    pub const fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }
}

/// Widget for rendering the errors section.
pub struct ErrorsSection;

impl StatefulWidget for ErrorsSection {
    type State = ErrorsSectionState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let border_color = if state.is_focused {
            Red::BRIGHT
        } else {
            Red::MUTED
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::new()
                    .fg(border_color)
                    .add_modifier(if state.is_focused {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )
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
