//! State node widget for functional tableau - renders a single expression step.

use std::iter::once;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget, Wrap},
};

use super::given_pane::binding_style_colors;
use crate::{
    protocol::{AnnotatedTextTree, BindingKind, GraphNode, LocalBinding},
    shared::{theme::Theme, truncate_str, ClickRegion, Selection},
};

/// State for a single functional state node widget.
#[derive(Default)]
pub struct FunctionalStateNodeState {
    pub click_regions: Vec<ClickRegion>,
}

/// A single functional expression node widget.
pub struct FunctionalStateNode<'a> {
    node: &'a GraphNode,
    is_current: bool,
    selection: Option<Selection>,
    top_down: bool,
}

impl<'a> FunctionalStateNode<'a> {
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

    fn is_leaf(&self) -> bool {
        self.node.edges.is_empty()
    }

    /// Check if node is complete (no expected type or has expected type that's
    /// satisfied).
    fn is_complete(&self) -> bool {
        self.expected_type().is_none() && self.is_leaf()
    }

    fn border_color(&self) -> Color {
        match (self.is_current, self.is_leaf(), self.is_complete()) {
            (true, _, _) => Theme::CURRENT_NODE_BORDER,
            (_, true, false) => Theme::INCOMPLETE_NODE_BORDER,
            (_, true, true) => Theme::COMPLETED_NODE_BORDER,
            _ => Theme::TACTIC_BORDER,
        }
    }

    fn title(&self) -> String {
        let expr_text = self.node.content.to_plain_text();
        match self.node.edges.len() {
            n if n > 1 => format!(" {expr_text} [{n}→] "),
            _ => format!(" {expr_text} "),
        }
    }

    fn is_binding_selected(&self, binding_idx: usize) -> bool {
        matches!(
            self.selection,
            Some(Selection::Hyp { node_id, hyp_idx }) if node_id == self.node.id && hyp_idx == binding_idx
        )
    }

    fn is_goal_selected(&self) -> bool {
        matches!(
            self.selection,
            Some(Selection::Goal { node_id, goal_idx: 0 }) if node_id == self.node.id
        )
    }

    /// Get bindings_after from node metadata.
    fn bindings_after(&self) -> Vec<LocalBinding> {
        self.node
            .metadata
            .get("bindings_after")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// Get new_binding_indices from node metadata.
    fn new_binding_indices(&self) -> Vec<u64> {
        self.node
            .metadata
            .get("new_binding_indices")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    /// Get expected_type from node metadata.
    fn expected_type(&self) -> Option<AnnotatedTextTree> {
        self.node
            .metadata
            .get("expected_type")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    fn bindings_line(&self) -> Option<Line<'static>> {
        let new_binding_indices = self.new_binding_indices();
        if new_binding_indices.is_empty() {
            return None;
        }

        let bindings_after = self.bindings_after();

        let spans: Vec<Span> = new_binding_indices
            .iter()
            .enumerate()
            .filter_map(|(i, &idx)| {
                let b = bindings_after.get(idx as usize)?;
                let (fg, bg) = binding_style_colors(b.binding_kind);
                let style = Style::new().fg(fg).bg(bg).add_modifier(
                    if self.is_binding_selected(idx as usize) {
                        Modifier::UNDERLINED
                    } else {
                        Modifier::empty()
                    },
                );

                let text = format!(
                    " {}: {} ",
                    b.name,
                    truncate_str(&b.type_.to_plain_text(), 20)
                );
                Some(if i > 0 {
                    vec![Span::raw(" "), Span::styled(text, style)]
                } else {
                    vec![Span::styled(text, style)]
                })
            })
            .flatten()
            .collect();

        (!spans.is_empty()).then(|| Line::from(spans))
    }

    fn type_line(&self) -> Line<'static> {
        let underline = if self.is_goal_selected() {
            Modifier::UNDERLINED
        } else {
            Modifier::empty()
        };

        match (self.expected_type(), self.is_leaf()) {
            (Some(ty), _) => Line::from(Span::styled(
                format!("⊢ {}", truncate_str(&ty.to_plain_text(), 35)),
                Style::new().fg(Theme::GOAL_FG).add_modifier(underline),
            )),
            (None, true) => Line::from(Span::styled(
                "✓ Complete",
                Style::new()
                    .fg(Theme::COMPLETED_GOAL_FG)
                    .add_modifier(Modifier::BOLD),
            )),
            (None, false) => Line::from(Span::styled(
                "⊢ ...",
                Style::new().fg(Theme::GOAL_FG).add_modifier(underline),
            )),
        }
    }

    fn click_regions(&self, inner: Rect) -> Vec<ClickRegion> {
        let new_binding_indices = self.new_binding_indices();
        let has_bindings = !new_binding_indices.is_empty();
        let offset = u16::from(has_bindings);
        let (bindings_y, type_y) = if self.top_down {
            (inner.y, inner.y + offset)
        } else {
            (inner.y + offset, inner.y)
        };

        let goal_region = ClickRegion {
            area: Rect::new(inner.x, type_y, inner.width, 1),
            selection: Selection::Goal {
                node_id: self.node.id,
                goal_idx: 0,
            },
        };

        let binding_regions = new_binding_indices.iter().map(|&idx| ClickRegion {
            area: Rect::new(inner.x, bindings_y, inner.width, 1),
            selection: Selection::Hyp {
                node_id: self.node.id,
                hyp_idx: idx as usize,
            },
        });

        once(goal_region).chain(binding_regions).collect()
    }
}

impl StatefulWidget for FunctionalStateNode<'_> {
    type State = FunctionalStateNodeState;

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

        let title_style = Style::new()
            .fg(if self.is_current {
                Color::White
            } else {
                Color::Gray
            })
            .add_modifier(if self.is_current {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(self.title(), title_style));

        // Only show continuation arrow for non-leaf nodes
        let block = if self.is_leaf() {
            block
        } else {
            let arrow = if self.top_down { "▼" } else { "▲" };
            block.title_bottom(Span::styled(format!(" {arrow} "), border_style))
        };

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        let bindings_line = self.bindings_line();
        let type_line = self.type_line();

        let lines: Vec<Line> = match (self.top_down, bindings_line) {
            (true, Some(b)) => vec![b, type_line],
            (false, Some(b)) => vec![type_line, b],
            (_, None) => vec![type_line],
        };

        Paragraph::new(lines)
            .wrap(Wrap { trim: true })
            .render(inner, buf);

        state.click_regions = self.click_regions(inner);
    }
}
