use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::{
    protocol::LocalBinding,
    selection::{Region, Selection},
    theme::{Blue, Gray, Theme},
};

#[derive(Default)]
pub struct InputPaneState {
    click_regions: Vec<Region>,
}

/// Input pane widget - displays block-level input bindings (theorem/def
/// parameters).
pub struct InputPane<'a> {
    bindings: &'a [LocalBinding],
    selection: Option<Selection>,
    is_focused: bool,
}

impl<'a> InputPane<'a> {
    pub const fn new(
        bindings: &'a [LocalBinding],
        selection: Option<Selection>,
        is_focused: bool,
    ) -> Self {
        Self {
            bindings,
            selection,
            is_focused,
        }
    }

    fn visible_bindings(&self) -> impl Iterator<Item = (usize, &LocalBinding)> {
        self.bindings
            .iter()
            .enumerate()
            .filter(|(_, b)| !b.is_implicit && !b.is_instance)
    }

    fn is_selected(&self, binding_idx: usize) -> bool {
        matches!(self.selection, Some(Selection::InitialHyp { hyp_idx }) if hyp_idx == binding_idx)
    }

    fn render_binding(&self, binding_idx: usize, binding: &LocalBinding) -> (Span<'static>, u16) {
        let style = Style::new()
            .fg(Theme::DATA_HYP_FG)
            .bg(Theme::DATA_HYP_BG)
            .add_modifier(if self.is_selected(binding_idx) {
                Modifier::UNDERLINED
            } else {
                Modifier::empty()
            });

        let text = format!(" {}: {} ", binding.name, binding.type_.to_plain_text());
        let width = text.chars().count() as u16;
        (Span::styled(text, style), width)
    }

    /// Calculate the required height to display all bindings, given the
    /// available width. Returns the content height (excluding borders).
    pub fn required_height(bindings: &[LocalBinding], available_width: u16) -> u16 {
        let visible: Vec<_> = bindings
            .iter()
            .filter(|b| !b.is_implicit && !b.is_instance)
            .collect();

        if visible.is_empty() {
            return 1; // "(no parameters)" text
        }

        // Calculate how many lines needed with wrapping
        let inner_width = available_width.saturating_sub(2) as usize; // Account for borders
        if inner_width == 0 {
            return 1;
        }

        let mut current_line_width = 0usize;
        let mut lines = 1u16;

        for binding in &visible {
            let text = format!(" {}: {} ", binding.name, binding.type_.to_plain_text());
            let binding_width = text.chars().count();

            if current_line_width == 0 {
                current_line_width = binding_width;
            } else if current_line_width + 1 + binding_width <= inner_width {
                current_line_width += 1 + binding_width; // +1 for separator
                                                         // space
            } else {
                lines += 1;
                current_line_width = binding_width;
            }
        }

        lines
    }
}

impl StatefulWidget for InputPane<'_> {
    type State = InputPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let border_color = if self.is_focused {
            Blue::BRIGHT
        } else if !self.bindings.is_empty() {
            Blue::MUTED
        } else {
            Gray::DIM
        };

        let title = if self.is_focused {
            "▶ Input "
        } else {
            " Input "
        };

        let title_color = if self.bindings.is_empty() {
            Gray::DIM
        } else {
            Theme::TITLE_ASSUMPTION
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

        let visible: Vec<_> = self.visible_bindings().collect();

        if visible.is_empty() {
            Paragraph::new("(no parameters)")
                .style(Style::new().fg(Color::DarkGray))
                .render(inner, buf);
            return;
        }

        let max_bindings = 5;
        let (spans, regions): (Vec<_>, Vec<_>) = visible
            .iter()
            .take(max_bindings)
            .scan(inner.x, |x_offset, (binding_idx, binding)| {
                let (span, width) = self.render_binding(*binding_idx, binding);
                let region = Region::from_rect(
                    Rect::new(*x_offset, inner.y, width.min(inner.width), 1),
                    Selection::InitialHyp {
                        hyp_idx: *binding_idx,
                    },
                );
                *x_offset += width + 1;
                Some(((span, *binding_idx != visible[0].0), region))
            })
            .map(|((span, needs_sep), region)| {
                let spans: Vec<Span> = if needs_sep {
                    vec![Span::raw(" "), span]
                } else {
                    vec![span]
                };
                (spans, region)
            })
            .unzip();

        state.click_regions.extend(regions);

        let mut all_spans: Vec<Span> = spans.into_iter().flatten().collect();
        if visible.len() > max_bindings {
            all_spans.push(Span::styled(
                format!(" +{}", visible.len() - max_bindings),
                Style::new().fg(Color::DarkGray),
            ));
        }

        Paragraph::new(Line::from(all_spans)).render(inner, buf);
    }
}
