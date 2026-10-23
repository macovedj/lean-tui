use std::iter::once;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

use crate::{
    protocol::RichText,
    theme::{Gray, Orange, Theme},
    widgets::diff_text::RichTextExt,
};

const TURNSTILE: &str = "⊢ ";

pub struct OutputPane<'a> {
    pub block_output: Option<&'a RichText>,
}

impl OutputPane<'_> {
    pub fn required_height(block_output: Option<&RichText>, available_width: u16) -> u16 {
        let inner_width = available_width.saturating_sub(2) as usize;
        let Some(output) = block_output.filter(|_| inner_width > 0) else {
            return 1;
        };
        let text_length = TURNSTILE.chars().count() + output.to_plain_text().chars().count();
        (text_length.div_ceil(inner_width) as u16).max(1)
    }
}

impl Widget for OutputPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let has_output = self.block_output.is_some();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(if has_output { Orange::MUTED } else { Gray::DIM }))
            .title(" Output ")
            .title_style(Style::new().fg(if has_output {
                Theme::TITLE_GOAL
            } else {
                Gray::DIM
            }));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 2 || inner.height < 1 {
            return;
        }

        let Some(output) = self.block_output else {
            Paragraph::new("(no output)")
                .style(Style::new().fg(Gray::DIM))
                .render(inner, buf);
            return;
        };

        let base_style = Style::new().fg(Theme::GOAL);
        let spans: Vec<Span> = once(Span::styled(
            TURNSTILE,
            base_style.add_modifier(Modifier::BOLD),
        ))
        .chain(output.to_spans(base_style))
        .collect();

        Paragraph::new(Line::from(spans))
            .wrap(Wrap { trim: false })
            .render(inner, buf);
    }
}
