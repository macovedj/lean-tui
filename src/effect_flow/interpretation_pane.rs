//! Interpretation pane for effect flow - shows forward/backward
//! interpretations.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};

use super::types::EffectFlowNodeExt;
use crate::{
    protocol::GraphNode,
    shared::{theme::Theme, truncate_str},
};

/// Render interpretation pane showing forward/backward for selected node.
pub fn render_interpretation_pane(node: Option<&GraphNode>, area: Rect, buf: &mut Buffer) {
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

    // Effect kind header
    lines.push(Line::from(vec![
        Span::styled(format!("{} ", kind.icon()), Style::new().fg(kind.color())),
        Span::styled(
            kind.description(),
            Style::new().fg(kind.color()).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" • {}", truncate_str(&node.content.to_plain_text(), 40)),
            Style::new().fg(Color::DarkGray),
        ),
    ]));

    // Forward/backward interpretations
    let mut interp_spans = Vec::new();
    if let Some(fwd) = node.forward() {
        interp_spans.push(Span::styled("fwd: ", Style::new().fg(Color::Green)));
        interp_spans.push(Span::styled(
            truncate_str(&fwd, 30),
            Style::new().fg(Color::White),
        ));
    }
    if let Some(bwd) = node.backward() {
        if !interp_spans.is_empty() {
            interp_spans.push(Span::raw("  "));
        }
        interp_spans.push(Span::styled("bwd: ", Style::new().fg(Color::Magenta)));
        interp_spans.push(Span::styled(
            truncate_str(&bwd, 30),
            Style::new().fg(Color::White),
        ));
    }
    if !interp_spans.is_empty() {
        lines.push(Line::from(interp_spans));
    }

    Paragraph::new(lines).render(inner, buf);
}
