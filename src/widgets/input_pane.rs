use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{
    protocol::LocalBinding,
    theme::{Blue, Gray, Theme},
};

const MAX_SHOWN_BINDINGS: usize = 5;

pub struct InputPane<'a> {
    pub bindings: &'a [LocalBinding],
}

const fn is_visible_binding(binding: &LocalBinding) -> bool {
    !binding.is_implicit && !binding.is_instance
}

fn binding_span(binding: &LocalBinding) -> Span<'static> {
    Span::styled(
        format!(" {}: {} ", binding.name, binding.type_.to_plain_text()),
        Style::new().fg(Theme::DATA_HYP_FG).bg(Theme::DATA_HYP_BG),
    )
}

impl InputPane<'_> {
    pub fn required_height(bindings: &[LocalBinding], available_width: u16) -> u16 {
        let inner_width = available_width.saturating_sub(2) as usize;
        let binding_widths: Vec<usize> = bindings
            .iter()
            .filter(|binding| is_visible_binding(binding))
            .map(|binding| binding_span(binding).content.chars().count())
            .collect();

        let Some((&first_width, rest_widths)) = binding_widths.split_first() else {
            return 1;
        };
        if inner_width == 0 {
            return 1;
        }

        let mut current_line_width = first_width;
        let mut lines = 1u16;
        for &binding_width in rest_widths {
            if current_line_width + 1 + binding_width <= inner_width {
                current_line_width += 1 + binding_width;
            } else {
                lines += 1;
                current_line_width = binding_width;
            }
        }
        lines
    }
}

impl Widget for InputPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let has_bindings = !self.bindings.is_empty();
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(if has_bindings { Blue::MUTED } else { Gray::DIM }))
            .title(" Input ")
            .title_style(Style::new().fg(if has_bindings {
                Theme::TITLE_ASSUMPTION
            } else {
                Gray::DIM
            }));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 2 || inner.height < 1 {
            return;
        }

        let visible: Vec<&LocalBinding> = self
            .bindings
            .iter()
            .filter(|binding| is_visible_binding(binding))
            .collect();

        if visible.is_empty() {
            Paragraph::new("(no parameters)")
                .style(Style::new().fg(Color::DarkGray))
                .render(inner, buf);
            return;
        }

        let hidden_count = visible.len().saturating_sub(MAX_SHOWN_BINDINGS);
        let spans: Vec<Span> = visible
            .iter()
            .take(MAX_SHOWN_BINDINGS)
            .enumerate()
            .flat_map(|(position, binding)| {
                let separator = (position > 0).then(|| Span::raw(" "));
                separator.into_iter().chain([binding_span(binding)])
            })
            .chain((hidden_count > 0).then(|| {
                Span::styled(
                    format!(" +{hidden_count}"),
                    Style::new().fg(Color::DarkGray),
                )
            }))
            .collect();

        Paragraph::new(Line::from(spans)).render(inner, buf);
    }
}
