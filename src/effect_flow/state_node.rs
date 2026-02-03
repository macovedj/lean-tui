use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use super::types::{EffectFlowNodeExt, EffectKind};
use crate::{
    protocol::GraphNode,
    shared::{theme::Theme, truncate_str, ClickRegion, Selection},
};

/// State for a single effect state node widget.
#[derive(Default)]
pub struct EffectStateNodeState {
    pub click_regions: Vec<ClickRegion>,
}

/// A single effect node widget with effect-kind-specific styling.
pub struct EffectStateNode<'a> {
    node: &'a GraphNode,
    is_current: bool,
    selection: Option<Selection>,
    top_down: bool,
}

impl<'a> EffectStateNode<'a> {
    pub const fn new(
        node: &'a GraphNode,
        is_current: bool,
        selection: Option<Selection>,
        top_down: bool,
    ) -> Self {
        Self {
            node,
            is_current,
            selection,
            top_down,
        }
    }

    fn effect_kind(&self) -> EffectKind {
        self.node.effect_kind()
    }

    fn is_leaf(&self) -> bool {
        self.node.edges.is_empty()
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

    fn is_goal_selected(&self) -> bool {
        matches!(
            self.selection,
            Some(Selection::Goal { node_id, goal_idx: 0 }) if node_id == self.node.id
        )
    }

    fn content_line(&self) -> Line<'static> {
        let underline = if self.is_goal_selected() {
            Modifier::UNDERLINED
        } else {
            Modifier::empty()
        };

        let kind = self.effect_kind();

        // Show different content based on effect kind
        match kind {
            EffectKind::StateRead => {
                let reads = self.node.bindings_read();
                if !reads.is_empty() {
                    return Line::from(Span::styled(
                        format!("◁ {}", reads.join(", ")),
                        Style::new()
                            .fg(Theme::EFFECT_STATE_READ)
                            .add_modifier(underline),
                    ));
                }
            }
            EffectKind::StateWrite => {
                let writes = self.node.bindings_written();
                if !writes.is_empty() {
                    return Line::from(Span::styled(
                        format!("▷ {}", writes.join(", ")),
                        Style::new()
                            .fg(Theme::EFFECT_STATE_WRITE)
                            .add_modifier(underline),
                    ));
                }
            }
            EffectKind::StateModify => {
                let reads = self.node.bindings_read();
                let writes = self.node.bindings_written();
                if !reads.is_empty() || !writes.is_empty() {
                    let text = format!(
                        "{}{}{}",
                        if reads.is_empty() {
                            String::new()
                        } else {
                            format!("◁{}", reads.join(","))
                        },
                        if !reads.is_empty() && !writes.is_empty() {
                            " "
                        } else {
                            ""
                        },
                        if writes.is_empty() {
                            String::new()
                        } else {
                            format!("▷{}", writes.join(","))
                        }
                    );
                    return Line::from(Span::styled(
                        text,
                        Style::new()
                            .fg(Theme::EFFECT_STATE_MODIFY)
                            .add_modifier(underline),
                    ));
                }
            }
            EffectKind::EffectThrow => {
                return Line::from(Span::styled(
                    "⚡ error",
                    Style::new()
                        .fg(Theme::EFFECT_THROW)
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(underline),
                ));
            }
            EffectKind::EffectCatch => {
                return Line::from(Span::styled(
                    "⛨ handler",
                    Style::new().fg(Theme::EFFECT_CATCH).add_modifier(underline),
                ));
            }
            _ => {}
        }

        // Default: show forward interpretation if available
        if let Some(fwd) = self.node.forward() {
            return Line::from(Span::styled(
                truncate_str(&fwd, 30),
                Style::new().fg(Color::White).add_modifier(underline),
            ));
        }

        // Fallback based on completion status
        if self.is_leaf() {
            Line::from(Span::styled(
                "✓",
                Style::new()
                    .fg(Theme::COMPLETED_GOAL_FG)
                    .add_modifier(Modifier::BOLD),
            ))
        } else {
            Line::from(Span::styled(
                "...",
                Style::new().fg(Color::DarkGray).add_modifier(underline),
            ))
        }
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

        let content_line = self.content_line();
        Paragraph::new(vec![content_line])
            .wrap(Wrap { trim: true })
            .render(inner, buf);

        state.click_regions = self.click_regions(inner);
    }
}
