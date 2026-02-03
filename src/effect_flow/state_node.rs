use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use super::types::{EffectFlowNode, EffectKind};
use crate::shared::{theme::Theme, ClickRegion, Selection};

/// State for a single effect state node widget.
#[derive(Default)]
pub struct EffectStateNodeState {
    pub click_regions: Vec<ClickRegion>,
}

/// A single effect node widget with effect-kind-specific styling.
pub struct EffectStateNode<'a> {
    node: &'a EffectFlowNode,
    is_current: bool,
    top_down: bool,
}

impl<'a> EffectStateNode<'a> {
    pub const fn new(node: &'a EffectFlowNode, is_current: bool, top_down: bool) -> Self {
        Self {
            node,
            is_current,
            top_down,
        }
    }

    fn effect_kind(&self) -> EffectKind {
        self.node.effect_kind()
    }

    fn is_leaf(&self) -> bool {
        self.node.is_leaf()
    }

    fn is_error_node(&self) -> bool {
        matches!(
            self.effect_kind(),
            EffectKind::EffectThrow | EffectKind::EffectCatch
        )
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
        let text_style = if self.is_current {
            Style::new().fg(Color::White).add_modifier(Modifier::BOLD)
        } else {
            Style::new().fg(Color::Gray)
        };

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

        // Backward interpretation (shown first/above)
        if let Some(bwd) = self.node.backward() {
            lines.push(Line::from(vec![
                Span::styled("bwd: ", Style::new().fg(Color::Magenta)),
                Span::raw(bwd),
            ]));
        }

        // Forward interpretation (shown second/below)
        if let Some(fwd) = self.node.forward() {
            lines.push(Line::from(vec![
                Span::styled("fwd: ", Style::new().fg(Color::Green)),
                Span::raw(fwd),
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

    fn click_regions(&self, inner: Rect) -> Vec<ClickRegion> {
        vec![ClickRegion {
            area: Rect::new(inner.x, inner.y, inner.width, inner.height.max(1)),
            selection: Selection::Goal {
                node_id: self.node.id,
                goal_idx: 0,
            },
        }]
    }
}

impl StatefulWidget for EffectStateNode<'_> {
    type State = EffectStateNodeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let border_color = self.border_color();
        let border_style = Style::new()
            .fg(border_color)
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

        // Show continuation arrow for non-leaf nodes
        let block = if self.is_leaf() {
            block
        } else {
            let arrow = if self.top_down { "▼" } else { "▲" };
            let arrow_style = if self.is_error_node() {
                Style::new().fg(Theme::EFFECT_THROW)
            } else {
                border_style
            };
            block.title_bottom(Span::styled(format!(" {arrow} "), arrow_style))
        };

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        Paragraph::new(self.content_lines()).render(inner, buf);

        state.click_regions = self.click_regions(inner);
    }
}
