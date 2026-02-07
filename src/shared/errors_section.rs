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
            (x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height)
                .then_some(*idx)
        });

        clicked_idx
            .and_then(|idx| self.errors.get(idx))
            .is_some_and(|error| {
                self.clicked_error = Some(error.message.clone());
                true
            })
    }

    /// Take the clicked error message (if any) and clear it.
    pub fn take_clicked_error(&mut self) -> Option<String> {
        self.clicked_error.take()
    }
}

/// Build styled error lines and a mapping from each line to its error index.
fn build_error_lines(errors: &[DiagnosticInfo]) -> (Vec<Line<'static>>, Vec<usize>) {
    errors
        .iter()
        .enumerate()
        .flat_map(|(error_idx, e)| {
            e.message.lines().enumerate().map(move |(i, line)| {
                let prefix = if i == 0 { "✗ " } else { "  " };
                let styled_line = Line::from(Span::styled(
                    format!("{prefix}{line}"),
                    Style::new().fg(Red::BRIGHT),
                ));
                (styled_line, error_idx)
            })
        })
        .unzip()
}

/// Save a click region if it has non-zero height.
fn save_region(
    regions: &mut Vec<(Rect, usize)>,
    x: u16,
    y: u16,
    width: u16,
    height: u16,
    idx: usize,
) {
    if height > 0 {
        regions.push((Rect::new(x, y, width, height), idx));
    }
}

/// Save the previous error region if transitioning to a new error.
fn save_previous_region(
    regions: &mut Vec<(Rect, usize)>,
    current_error_idx: Option<usize>,
    current_y: u16,
    region_start_y: u16,
    inner: Rect,
) {
    let Some(prev_idx) = current_error_idx else {
        return;
    };
    let height = current_y - region_start_y;
    save_region(
        regions,
        inner.x,
        region_start_y,
        inner.width,
        height,
        prev_idx,
    );
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

        let (error_lines, line_to_error) = build_error_lines(&state.errors);

        // Track click regions for each error
        let mut current_y = inner.y;
        let mut current_error_idx = None;
        let mut region_start_y = inner.y;

        for (line_idx, &error_idx) in line_to_error.iter().enumerate() {
            if current_y >= inner.y + inner.height {
                break;
            }

            if current_error_idx != Some(error_idx) {
                save_previous_region(
                    &mut state.click_regions,
                    current_error_idx,
                    current_y,
                    region_start_y,
                    inner,
                );
                current_error_idx = Some(error_idx);
                region_start_y = current_y;
            }

            let line_chars = error_lines.get(line_idx).map_or(0, Line::width);
            let wrapped_lines = ((line_chars as u16).saturating_sub(1) / inner.width) + 1;
            current_y += wrapped_lines;
        }

        // Save final region
        let final_y = current_y.min(inner.y + inner.height);
        save_previous_region(
            &mut state.click_regions,
            current_error_idx,
            final_y,
            region_start_y,
            inner,
        );

        Paragraph::new(error_lines)
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}
