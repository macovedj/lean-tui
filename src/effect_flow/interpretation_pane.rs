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

    let kind = &node.effect_kind;
    let mut lines = Vec::new();

    // Effect kind header with expression
    let mut header_spans = vec![
        Span::styled(format!("{} ", kind.icon()), Style::new().fg(kind.color())),
        Span::styled(
            kind.description(),
            Style::new().fg(kind.color()).add_modifier(Modifier::BOLD),
        ),
    ];

    // Show state effect type if present
    if let Some(state_eff) = &node.state_effect {
        header_spans.push(Span::styled(
            format!(" [{state_eff}]"),
            Style::new().fg(Color::Yellow),
        ));
    }

    header_spans.push(Span::styled(
        format!(" • {}", node.content.to_plain_text()),
        Style::new().fg(Color::DarkGray),
    ));
    lines.push(Line::from(header_spans));

    // Forward interpretation - what the operation returns (Aeneas "get")
    if let Some(fwd) = &node.forward {
        lines.push(Line::from(vec![
            Span::styled("→ returns: ", Style::new().fg(Color::Green)),
            Span::raw(fwd.clone()),
        ]));
    }

    // Show symbolic forward description if different from output
    if let (Some(desc), Some(output)) = (&node.forward_desc, &node.forward_output) {
        if desc != output && !desc.is_empty() && !output.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::new()),
                Span::styled(desc.clone(), Style::new().fg(Color::DarkGray)),
            ]));
        }
    }

    // Backward interpretation - what gets propagated back (Aeneas "put")
    if let Some(bwd) = &node.backward {
        lines.push(Line::from(vec![
            Span::styled("← gives back: ", Style::new().fg(Color::Magenta)),
            Span::raw(bwd.clone()),
        ]));
    }

    // Show symbolic backward description if different from given_back
    if let (Some(desc), Some(given)) = (&node.backward_desc, &node.given_back) {
        if desc != given && !desc.is_empty() && !given.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("  ", Style::new()),
                Span::styled(desc.clone(), Style::new().fg(Color::DarkGray)),
            ]));
        }
    }

    Paragraph::new(lines).render(inner, buf);
}
