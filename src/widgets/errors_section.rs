use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::{
    message::{AppMessage, ClickTarget},
    protocol::DiagnosticInfo,
    theme::Red,
};

struct ErrorLine {
    line: Line<'static>,
    error_index: usize,
}

fn build_error_lines(errors: &[DiagnosticInfo]) -> Vec<ErrorLine> {
    errors
        .iter()
        .enumerate()
        .flat_map(|(error_index, error)| {
            error.message.lines().enumerate().map(move |(i, line)| {
                let prefix = if i == 0 { "✗ " } else { "  " };
                ErrorLine {
                    line: Line::from(Span::styled(
                        format!("{prefix}{line}"),
                        Style::new().fg(Red::BRIGHT),
                    )),
                    error_index,
                }
            })
        })
        .collect()
}

struct ErrorRows {
    error_index: usize,
    first_row: u16,
    row_count: u16,
}

fn error_row_spans(error_lines: &[ErrorLine], inner: Rect) -> Vec<ErrorRows> {
    let bottom = inner.y + inner.height;
    let mut spans: Vec<ErrorRows> = Vec::new();
    let mut row = inner.y;
    for error_line in error_lines {
        if row >= bottom {
            break;
        }
        let wrapped_rows = ((error_line.line.width() as u16).saturating_sub(1) / inner.width) + 1;
        let visible_rows = wrapped_rows.min(bottom - row);
        match spans.last_mut() {
            Some(last) if last.error_index == error_line.error_index => {
                last.row_count += visible_rows;
            }
            _ => spans.push(ErrorRows {
                error_index: error_line.error_index,
                first_row: row,
                row_count: visible_rows,
            }),
        }
        row += wrapped_rows;
    }
    spans
}

pub struct ErrorsSection<'a> {
    pub errors: &'a [DiagnosticInfo],
}

impl ErrorsSection<'_> {
    pub fn draw(self, area: Rect, buf: &mut Buffer) -> Vec<ClickTarget> {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Red::MUTED))
            .title(Span::styled(
                " Errors ",
                Style::new().fg(Red::BRIGHT).add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 2 || inner.height < 1 || self.errors.is_empty() {
            return Vec::new();
        }

        let error_lines = build_error_lines(self.errors);
        let click_targets = error_row_spans(&error_lines, inner)
            .into_iter()
            .filter(|rows| rows.row_count > 0)
            .filter_map(|rows| {
                let error = self.errors.get(rows.error_index)?;
                Some(ClickTarget {
                    area: Rect::new(inner.x, rows.first_row, inner.width, rows.row_count),
                    message: AppMessage::ShowError {
                        message: error.message.clone(),
                    },
                })
            })
            .collect();

        Paragraph::new(
            error_lines
                .into_iter()
                .map(|error_line| error_line.line)
                .collect::<Vec<_>>(),
        )
        .wrap(Wrap { trim: false })
        .render(inner, buf);
        click_targets
    }
}
