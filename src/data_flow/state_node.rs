use std::iter::once;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use super::types::{DataFlowKind, DataFlowNode, SimpleBinding};
use crate::{
    protocol::BindingKind,
    shared::{theme::Theme, ClickRegion, Selection},
};

/// State for a single functional state node widget.
#[derive(Default)]
pub struct FunctionalStateNodeState {
    pub click_regions: Vec<ClickRegion>,
}

/// A single functional expression node widget.
pub struct FunctionalStateNode<'a> {
    node: &'a DataFlowNode,
    is_current: bool,
    selection: Option<Selection>,
    top_down: bool,
}

impl<'a> FunctionalStateNode<'a> {
    pub const fn new(
        node: &'a DataFlowNode,
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

    fn border_color(&self) -> Color {
        match (
            self.is_current,
            self.node.is_leaf(),
            self.node.is_complete(),
        ) {
            (true, _, _) => Theme::CURRENT_NODE_BORDER,
            (_, true, false) => Theme::INCOMPLETE_NODE_BORDER,
            (_, true, true) => Theme::COMPLETED_NODE_BORDER,
            _ => Theme::TACTIC_BORDER,
        }
    }

    fn node_kind(&self) -> DataFlowKind {
        self.node.data_flow_kind()
    }

    fn title(&self) -> String {
        let kind = self.node_kind();
        let icon = kind.icon();
        // Use branch label if available, otherwise use expression content
        let expr_text = self
            .node
            .branch_label()
            .unwrap_or_else(|| self.node.content.to_plain_text());
        match self.node.edges.len() {
            n if n > 1 => format!(" {icon} {expr_text} [{n}→] "),
            _ => format!(" {icon} {expr_text} "),
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

    fn bindings_line(&self) -> Option<Line<'static>> {
        let bindings = self.node.bindings_after();
        if bindings.is_empty() {
            return None;
        }

        let spans: Vec<Span> = bindings
            .iter()
            .enumerate()
            .flat_map(|(idx, b)| {
                let is_new = self.node.is_new_binding(idx);
                let is_selected = self.is_binding_selected(idx);
                let style = binding_style(b, is_new, is_selected);

                let text = format!(" {}: {} ", b.name, b.type_str);
                if idx > 0 {
                    vec![Span::raw(" "), Span::styled(text, style)]
                } else {
                    vec![Span::styled(text, style)]
                }
            })
            .collect();

        (!spans.is_empty()).then(|| Line::from(spans))
    }

    fn type_line(&self) -> Line<'static> {
        let underline = if self.is_goal_selected() {
            Modifier::UNDERLINED
        } else {
            Modifier::empty()
        };

        match (self.node.expected_type(), self.node.is_complete()) {
            (Some(ty), _) => Line::from(Span::styled(
                format!("⊢ {ty}"),
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
        let bindings = self.node.bindings_after();
        let has_bindings = !bindings.is_empty();
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

        let binding_regions = (0..bindings.len()).map(|idx| ClickRegion {
            area: Rect::new(inner.x, bindings_y, inner.width, 1),
            selection: Selection::Hyp {
                node_id: self.node.id,
                hyp_idx: idx,
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

        let kind_color = self.node_kind().color();
        let title_style = Style::new()
            .fg(if self.is_current {
                kind_color
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

        // Render without wrapping - content will be clipped if too wide
        Paragraph::new(lines).render(inner, buf);

        state.click_regions = self.click_regions(inner);
    }
}

/// Get style for a binding based on its kind, whether it's new, and whether
/// it's selected.
fn binding_style(binding: &SimpleBinding, is_new: bool, is_selected: bool) -> Style {
    let (fg, bg) = binding_colors(binding.kind(), is_new);
    let mut style = Style::new().fg(fg).bg(bg);
    if is_selected {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if is_new {
        style = style.add_modifier(Modifier::BOLD);
    }
    style
}

/// Get colors for a binding based on kind and whether it's newly introduced.
fn binding_colors(kind: BindingKind, is_new: bool) -> (Color, Color) {
    if is_new {
        // New bindings get brighter/highlighted colors
        match kind {
            BindingKind::LetBind => (Theme::PROOF_HYP_FG, Theme::PROOF_HYP_BG),
            BindingKind::FunParam => (Theme::DATA_HYP_FG, Theme::DATA_HYP_BG),
            BindingKind::MatchVar => (Color::Rgb(200, 170, 220), Color::Rgb(50, 45, 60)),
            BindingKind::ForVar => (Color::Rgb(170, 200, 220), Color::Rgb(45, 50, 60)),
        }
    } else {
        // Existing bindings get dimmer colors
        match kind {
            BindingKind::LetBind => (Color::Rgb(100, 90, 80), Color::Reset),
            BindingKind::FunParam => (Color::Rgb(80, 100, 90), Color::Reset),
            BindingKind::MatchVar => (Color::Rgb(100, 90, 110), Color::Reset),
            BindingKind::ForVar => (Color::Rgb(90, 100, 110), Color::Reset),
        }
    }
}
