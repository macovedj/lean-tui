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
    click_regions: Vec<(Rect, usize)>,
    clicked_error: Option<String>,
}

impl ErrorsSectionState {
    pub fn update(&mut self, errors: Vec<DiagnosticInfo>) {
        self.errors = errors;
    }

    /// Handle a click at (x, y). Returns true if an error was clicked.
    pub fn handle_click(&mut self, x: u16, y: u16) -> bool {
        let clicked_idx = self.click_regions.iter().find_map(|(rect, idx)| {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                Some(*idx)
            } else {
                None
            }
        });

        if let Some(idx) = clicked_idx {
            if let Some(error) = self.errors.get(idx) {
                self.clicked_error = Some(error.message.clone());
                return true;
            }
        }
        false
    }

    /// Take the clicked error message (if any) and clear it.
    pub fn take_clicked_error(&mut self) -> Option<String> {
        self.clicked_error.take()
    }
}

/// Widget for rendering the errors section.
pub struct ErrorsSection;

impl StatefulWidget for ErrorsSection {
    type State = ErrorsSectionState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

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

        // Build lines and track which error each line belongs to
        let mut error_lines: Vec<Line> = Vec::new();
        let mut line_to_error: Vec<usize> = Vec::new();

        for (error_idx, e) in state.errors.iter().enumerate() {
            for (i, line) in e.message.lines().enumerate() {
                let prefix = if i == 0 { "✗ " } else { "  " };
                error_lines.push(Line::from(Span::styled(
                    format!("{prefix}{line}"),
                    Style::new().fg(Red::BRIGHT),
                )));
                line_to_error.push(error_idx);
            }
        }

        // Track click regions for each error (map rendered lines to errors)
        let mut current_y = inner.y;
        let mut current_error_idx = None;
        let mut region_start_y = inner.y;

        for (line_idx, &error_idx) in line_to_error.iter().enumerate() {
            if current_y >= inner.y + inner.height {
                break;
            }

            if current_error_idx != Some(error_idx) {
                // Save previous region if any
                if let Some(prev_idx) = current_error_idx {
                    let height = current_y - region_start_y;
                    if height > 0 {
                        state.click_regions.push((
                            Rect::new(inner.x, region_start_y, inner.width, height),
                            prev_idx,
                        ));
                    }
                }
                current_error_idx = Some(error_idx);
                region_start_y = current_y;
            }

            // Account for wrapped lines
            let line_chars = error_lines.get(line_idx).map_or(0, |l| l.width());
            let wrapped_lines = ((line_chars as u16).saturating_sub(1) / inner.width) + 1;
            current_y += wrapped_lines;
        }

        // Save final region
        if let Some(prev_idx) = current_error_idx {
            let height = (current_y.min(inner.y + inner.height)) - region_start_y;
            if height > 0 {
                state.click_regions.push((
                    Rect::new(inner.x, region_start_y, inner.width, height),
                    prev_idx,
                ));
            }
        }

        Paragraph::new(error_lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}
