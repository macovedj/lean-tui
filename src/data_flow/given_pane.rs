//! Given pane for functional tableau - displays initial bindings (function
//! parameters).

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use crate::{
    protocol::{BindingKind, LocalBinding},
    shared::{theme::Theme, truncate_str, ClickRegion, Selection},
};

/// State for the binding given pane.
#[derive(Default)]
pub struct BindingGivenPaneState {
    pub click_regions: Vec<ClickRegion>,
}

impl BindingGivenPaneState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(Rect::new(x, y, 1, 1)))
            .map(|r| r.selection)
    }
}

/// Given pane widget for functional DAG - displays initial bindings (function
/// parameters).
pub struct BindingGivenPane<'a> {
    bindings: &'a [LocalBinding],
    selection: Option<Selection>,
}

impl<'a> BindingGivenPane<'a> {
    pub const fn new(bindings: &'a [LocalBinding], selection: Option<Selection>) -> Self {
        Self {
            bindings,
            selection,
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
        let (fg, bg) = binding_style_colors(binding.binding_kind);
        let style = Style::new()
            .fg(fg)
            .bg(bg)
            .add_modifier(if self.is_selected(binding_idx) {
                Modifier::UNDERLINED
            } else {
                Modifier::empty()
            });

        let text = format!(
            " {}: {} ",
            binding.name,
            truncate_str(&binding.type_.to_plain_text(), 20)
        );
        let width = text.chars().count() as u16;
        (Span::styled(text, style), width)
    }
}

impl StatefulWidget for BindingGivenPane<'_> {
    type State = BindingGivenPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Theme::DATA_HYP_FG))
            .title(Span::styled(
                " Given ",
                Style::new()
                    .fg(Theme::DATA_HYP_FG)
                    .add_modifier(Modifier::BOLD),
            ));

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

        // Build spans and click regions together
        let (spans, regions): (Vec<_>, Vec<_>) = visible
            .iter()
            .take(5)
            .scan(inner.x, |x_offset, (binding_idx, binding)| {
                let (span, width) = self.render_binding(*binding_idx, binding);
                let region = ClickRegion {
                    area: Rect::new(*x_offset, inner.y, width.min(inner.width), 1),
                    selection: Selection::InitialHyp {
                        hyp_idx: *binding_idx,
                    },
                };
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

        state.click_regions = regions;

        let mut all_spans: Vec<Span> = spans.into_iter().flatten().collect();
        if visible.len() > 5 {
            all_spans.push(Span::styled(
                format!(" +{}", visible.len() - 5),
                Style::new().fg(Color::DarkGray),
            ));
        }

        Paragraph::new(Line::from(all_spans)).render(inner, buf);
    }
}

/// Get binding style colors based on binding kind.
pub const fn binding_style_colors(kind: BindingKind) -> (Color, Color) {
    match kind {
        BindingKind::LetBind => (Theme::PROOF_HYP_FG, Theme::PROOF_HYP_BG),
        BindingKind::FunParam => (Theme::DATA_HYP_FG, Theme::DATA_HYP_BG),
        BindingKind::MatchVar => (Color::Rgb(180, 150, 200), Color::Rgb(40, 35, 50)),
        BindingKind::ForVar => (Color::Rgb(150, 180, 200), Color::Rgb(35, 40, 50)),
    }
}
