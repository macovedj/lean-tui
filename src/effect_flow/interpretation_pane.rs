//! Interpretation pane for effect flow.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::types::EffectFlowNode;
use crate::shared::theme::Theme;

/// Render interpretation pane showing backward/forward for selected node.
pub fn render_interpretation_pane(node: Option<&EffectFlowNode>, area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Theme::BORDER))
        .title(Span::styled(
            " Interpretations ",
            Style::new().fg(Color::Cyan),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    let Some(node) = node else {
        Paragraph::new(Span::styled(
            "(select a node)",
            Style::new().fg(Color::DarkGray),
        ))
        .render(inner, buf);
        return;
    };

    let kind = node.effect_kind();
    let mut lines = Vec::new();

    // Effect kind header with expression
    lines.push(Line::from(vec![
        Span::styled(format!("{} ", kind.icon()), Style::new().fg(kind.color())),
        Span::styled(
            kind.description(),
            Style::new().fg(kind.color()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" • {}", node.content.to_plain_text()),
            Style::new().fg(Color::DarkGray),
        ),
    ]));

    // Backward interpretation (above forward)
    if let Some(bwd) = node.backward() {
        lines.push(Line::from(vec![
            Span::styled("bwd: ", Style::new().fg(Color::Magenta)),
            Span::raw(bwd),
        ]));
    }

    // Forward interpretation
    if let Some(fwd) = node.forward() {
        lines.push(Line::from(vec![
            Span::styled("fwd: ", Style::new().fg(Color::Green)),
            Span::raw(fwd),
        ]));
    }

    Paragraph::new(lines).render(inner, buf);
}
