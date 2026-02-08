use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use crate::{
    protocol::RichText,
    shared::{
        diff_text::RichTextExt,
        theme::{Gray, Orange, Theme},
        Region, Selection,
    },
};

#[derive(Default)]
pub struct OutputPaneState {
    click_regions: Vec<Region>,
}

/// Output pane widget - displays block-level output (theorem statement).
pub struct OutputPane<'a> {
    block_output: Option<&'a RichText>,
    selection: Option<Selection>,
    is_focused: bool,
}

impl<'a> OutputPane<'a> {
    pub const fn new(
        block_output: Option<&'a RichText>,
        selection: Option<Selection>,
        is_focused: bool,
    ) -> Self {
        Self {
            block_output,
            selection,
            is_focused,
        }
    }

    fn is_selected(&self) -> bool {
        matches!(self.selection, Some(Selection::Theorem))
    }

    /// Calculate the required height to display the output, given the available
    /// width. Returns the content height (excluding borders).
    pub fn required_height(block_output: Option<&RichText>, available_width: u16) -> u16 {
        let Some(output) = block_output else {
            return 1; // "(no output)" text
        };

        let inner_width = available_width.saturating_sub(2) as usize; // Account for borders
        if inner_width == 0 {
            return 1;
        }

        // Text is "⊢ " + the output
        let text = format!("⊢ {}", output.to_plain_text());
        let text_len = text.chars().count();

        // Calculate lines needed with wrapping
        let lines = text_len.div_ceil(inner_width);
        (lines as u16).max(1)
    }
}

impl StatefulWidget for OutputPane<'_> {
    type State = OutputPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let has_output = self.block_output.is_some();

        let border_color = if self.is_focused {
            Orange::BRIGHT
        } else if has_output {
            Orange::MUTED
        } else {
            Gray::DIM
        };

        let title = if self.is_focused {
            "▶ Output "
        } else {
            " Output "
        };

        let title_color = if has_output {
            Theme::TITLE_GOAL
        } else {
            Gray::DIM
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(title)
            .title_style(Style::new().fg(title_color));

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

        let base_style = if self.is_selected() {
            Style::new().fg(Theme::GOAL).bg(Theme::SELECTION_BG)
        } else {
            Style::new().fg(Theme::GOAL)
        };

        let turnstile_style = base_style.add_modifier(Modifier::BOLD);

        let mut spans = vec![ratatui::text::Span::styled("⊢ ", turnstile_style)];
        spans.extend(output.to_spans(base_style));

        let paragraph = Paragraph::new(ratatui::text::Line::from(spans)).wrap(Wrap { trim: false });
        paragraph.render(inner, buf);

        // Register click region for theorem selection
        state
            .click_regions
            .push(Region::from_rect(inner, Selection::Theorem));
    }
}
