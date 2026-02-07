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
}

impl ErrorsSectionState {
    pub fn update(&mut self, errors: Vec<DiagnosticInfo>) {
        self.errors = errors;
    }

    pub fn errors(&self) -> &[DiagnosticInfo] {
        &self.errors
    }

    /// Find error index at click position.
    pub fn find_error_at(&self, x: u16, y: u16) -> Option<usize> {
        self.click_regions.iter().find_map(|(rect, idx)| {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                Some(*idx)
            } else {
                None
            }
        })
    }

    /// Get error message by index.
    pub fn get_error_message(&self, idx: usize) -> Option<&str> {
        self.errors.get(idx).map(|e| e.message.as_str())
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
