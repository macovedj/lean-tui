use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use super::types::{EffectFlowNode, EffectKind};
use crate::shared::{theme::Theme, Region, Selection};

/// State for a single effect state node widget.
#[derive(Default)]
pub struct EffectStateNodeState {
    pub click_regions: Vec<Region>,
}

/// A single effect node widget with effect-kind-specific styling.
pub struct EffectStateNode<'a> {
    node: &'a EffectFlowNode,
    is_current: bool,
    is_selected: bool,
}

impl<'a> EffectStateNode<'a> {
    pub const fn new(node: &'a EffectFlowNode, is_current: bool, is_selected: bool) -> Self {
        Self {
            node,
            is_current,
            is_selected,
        }
    }

    fn effect_kind(&self) -> &EffectKind {
        &self.node.effect_kind
    }

    fn is_leaf(&self) -> bool {
        self.node.is_leaf()
    }

    fn border_color(&self) -> Color {
        if self.is_current {
            return Theme::CURRENT_NODE_BORDER;
        }

        let kind = self.effect_kind();

        // Use effect-specific colors for borders
        match kind {
            EffectKind::EffectThrow => Theme::EFFECT_THROW,
            EffectKind::EffectCatch => Theme::EFFECT_CATCH,
            EffectKind::StateRead | EffectKind::StateWrite | EffectKind::StateModify => {
                if self.is_leaf() {
                    Theme::COMPLETED_NODE_BORDER
                } else {
                    kind.color()
                }
            }
            _ => {
                if self.is_leaf() {
                    Theme::COMPLETED_NODE_BORDER
                } else {
                    Theme::TACTIC_BORDER
                }
            }
        }
    }

    fn title(&self) -> Vec<Span<'static>> {
        let kind = self.effect_kind();
        let expr_text = self.node.content.to_plain_text();

        let icon_style = Style::new().fg(kind.color());
        let mut text_style = if self.is_current {
            Style::new().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(Color::Gray)
        };

        if self.is_selected {
            text_style = text_style.add_modifier(Modifier::UNDERLINED);
        }

        let mut spans = vec![
            Span::styled(format!(" {} ", kind.icon()), icon_style),
            Span::styled(expr_text, text_style),
        ];

        // Show edge count if multiple outgoing edges
        if self.node.edges.len() > 1 {
            spans.push(Span::styled(
                format!(" [{}→]", self.node.edges.len()),
                Style::new().fg(Color::DarkGray),
            ));
        }

        spans.push(Span::raw(" "));
        spans
    }

    fn content_lines(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Return component - what the operation produces
        if let Some(fwd) = &self.node.forward {
            lines.push(Line::from(vec![
                Span::styled("→ ", Style::new().fg(Color::Green)),
                Span::raw(fwd.clone()),
            ]));
        }

        // Effect component - what state changes are given back
        if let Some(bwd) = &self.node.backward {
            lines.push(Line::from(vec![
                Span::styled("← ", Style::new().fg(Color::Magenta)),
                Span::raw(bwd.clone()),
            ]));
        }

        if lines.is_empty() && self.is_leaf() {
            lines.push(Line::from(Span::styled(
                "✓",
                Style::new()
                    .fg(Theme::COMPLETED_GOAL_FG)
                    .add_modifier(Modifier::BOLD),
            )));
        }

        lines
    }

    fn click_regions(&self, area: Rect) -> Vec<Region> {
        // Click region is just the header row (top border with title)
        vec![Region::from_rect(
            Rect::new(area.x, area.y, area.width, 1),
            Selection::Goal {
                node_id: self.node.id,
                goal_idx: 0,
            },
        )]
    }
}

impl StatefulWidget for EffectStateNode<'_> {
    type State = EffectStateNodeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        // Fill background to make node opaque (hides edges behind it)
        let bg_color = Theme::NODE_BG;
        for y in area.y..area.y + area.height {
            for x in area.x..area.x + area.width {
                if let Some(cell) = buf.cell_mut((x, y)) {
                    cell.set_bg(bg_color);
                }
            }
        }

        let border_color = self.border_color();
        let border_style =
            Style::new()
                .fg(border_color)
                .bg(bg_color)
                .add_modifier(if self.is_current {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                });

        let title_spans = self.title();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Line::from(title_spans));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        Paragraph::new(self.content_lines()).render(inner, buf);

        state.click_regions = self.click_regions(area);
    }
}
