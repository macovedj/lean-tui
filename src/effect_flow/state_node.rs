use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use super::types::{EffectFlowNode, EffectKind};
use crate::{
    protocol::{DiagnosticInfo, DiagnosticSeverity},
    selection::{Region, Selection},
    theme::{Red, Theme},
    widgets::{
        aligned_table::{AlignedRow, AlignedTable},
        diff_text::RichTextExt,
    },
};

/// State for a single effect state node widget.
#[derive(Default)]
pub struct EffectStateNodeState {
    pub click_regions: Vec<Region>,
}

/// A single effect node widget with effect-kind-specific styling.
pub struct EffectStateNode<'a> {
    node: &'a EffectFlowNode,
    is_current: bool,
    selection: Option<Selection>,
}

impl<'a> EffectStateNode<'a> {
    pub const fn new(
        node: &'a EffectFlowNode,
        is_current: bool,
        selection: Option<Selection>,
    ) -> Self {
        Self {
            node,
            is_current,
            selection,
        }
    }

    fn is_hyp_selected(&self, hyp_idx: usize) -> bool {
        matches!(
            self.selection,
            Some(Selection::Hyp { node_id, hyp_idx: hi }) if node_id == self.node.id && hi == hyp_idx
        )
    }

    fn effect_kind(&self) -> &EffectKind {
        &self.node.effect_kind
    }

    fn is_leaf(&self) -> bool {
        self.node.is_leaf()
    }

    fn border_color(&self) -> Color {
        // Error nodes get red border regardless of other state
        if self.node.has_error() {
            return Red::BRIGHT;
        }

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
                    Theme::INACTIVE_NODE_BORDER
                }
            }
        }
    }

    fn title(&self) -> Vec<Span<'static>> {
        let kind = self.effect_kind();
        let has_error = self.node.has_error();

        let mut spans = Vec::new();

        // Icon in the title (top-left corner)
        let icon_style = if has_error {
            Style::new().fg(Red::BRIGHT)
        } else {
            Style::new().fg(kind.color())
        };
        spans.push(Span::styled(format!(" {}", kind.icon()), icon_style));

        if has_error {
            spans.push(Span::styled(" ⚠", Style::new().fg(Red::BRIGHT)));
        }

        spans.push(Span::raw(" "));
        spans
    }

    fn build_expression_line(&self) -> Line<'static> {
        let has_error = self.node.has_error();

        // Use ITALIC to distinguish expression from goals (which use bold/normal)
        let style = if has_error {
            Style::new().fg(Red::BRIGHT).add_modifier(Modifier::ITALIC)
        } else if self.is_current {
            Style::new().fg(Color::White).add_modifier(Modifier::ITALIC)
        } else {
            Style::new().fg(Color::Gray).add_modifier(Modifier::ITALIC)
        };

        Line::from(self.node.content.to_spans(style))
    }

    fn build_binding_rows(&self) -> Vec<AlignedRow> {
        self.node
            .visible_bindings()
            .into_iter()
            .enumerate()
            .map(|(idx, b)| {
                let underline = if self.is_hyp_selected(idx) {
                    Modifier::UNDERLINED
                } else {
                    Modifier::empty()
                };
                let name_style = Style::new().fg(Theme::HYP_NAME).add_modifier(underline);
                let type_style = Style::new().fg(Color::White).add_modifier(underline);
                AlignedRow::with_styled_type(
                    b.name.clone(),
                    name_style,
                    ":",
                    Style::new().fg(Color::DarkGray),
                    b.type_.to_spans(type_style),
                )
            })
            .collect()
    }

    fn build_effect_rows(&self) -> Vec<AlignedRow> {
        let mut rows = Vec::new();

        for fwd in &self.node.forwards {
            rows.push(AlignedRow::new(
                "",
                Style::default(),
                "→",
                Style::new().fg(Color::Green),
                fwd.value.clone(),
                Style::new().fg(Color::White),
            ));
        }

        for bwd in &self.node.backwards {
            rows.push(AlignedRow::new(
                "",
                Style::default(),
                "←",
                Style::new().fg(Color::Magenta),
                bwd.value.clone(),
                Style::new().fg(Color::White),
            ));
        }

        if rows.is_empty() && self.is_leaf() && !self.node.has_error() {
            rows.push(AlignedRow::new(
                "",
                Style::default(),
                "✓",
                Style::new()
                    .fg(Theme::COMPLETED_GOAL_FG)
                    .add_modifier(Modifier::BOLD),
                "completed",
                Style::new()
                    .fg(Theme::COMPLETED_GOAL_FG)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        rows
    }

    fn click_regions(&self, inner: Rect, error_count: usize) -> Vec<Region> {
        let mut regions = Vec::new();
        let binding_count = self.node.visible_bindings().len();
        let effect_count = self.node.forwards.len() + self.node.backwards.len();
        let mut current_y = inner.y;

        // Bindings
        for hyp_idx in 0..binding_count {
            if current_y < inner.y + inner.height {
                regions.push(Region::from_rect(
                    Rect::new(inner.x, current_y, inner.width, 1),
                    Selection::Hyp { node_id: self.node.id, hyp_idx },
                ));
            }
            current_y += 1;
        }

        // Expression (Goal)
        if current_y < inner.y + inner.height {
            regions.push(Region::from_rect(
                Rect::new(inner.x, current_y, inner.width, 1),
                Selection::Goal { node_id: self.node.id, goal_idx: 0 },
            ));
        }
        current_y += 1;

        // Skip effect rows
        current_y += effect_count as u16;

        // Errors
        for error_idx in 0..error_count {
            if current_y < inner.y + inner.height {
                regions.push(Region::from_rect(
                    Rect::new(inner.x, current_y, inner.width, 1),
                    Selection::Error { node_id: self.node.id, error_idx },
                ));
            }
            current_y += 1;
        }

        regions
    }
}

impl StatefulWidget for EffectStateNode<'_> {
    type State = EffectStateNodeState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let border_color = self.border_color();
        let border_style = Style::new().fg(border_color).add_modifier(
            if self.is_current || self.node.has_error() {
                Modifier::BOLD
            } else {
                Modifier::empty()
            },
        );

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

        let mut y = inner.y;

        // 1. Render bindings table
        let binding_rows = self.build_binding_rows();
        if !binding_rows.is_empty() {
            let height = (binding_rows.len() as u16).min(inner.height);
            let bindings_area = Rect::new(inner.x, y, inner.width, height);
            Widget::render(AlignedTable::new(binding_rows), bindings_area, buf);
            y += height;
        }

        // 2. Render expression line
        if y < inner.y + inner.height {
            buf.set_line(inner.x, y, &self.build_expression_line(), inner.width);
            y += 1;
        }

        // 3. Render effects table
        let effect_rows = self.build_effect_rows();
        if !effect_rows.is_empty() && y < inner.y + inner.height {
            let effects_height = (effect_rows.len() as u16).min(inner.y + inner.height - y);
            let effects_area = Rect::new(inner.x, y, inner.width, effects_height);
            Widget::render(AlignedTable::new(effect_rows), effects_area, buf);
            y += effects_height;
        }

        // 4. Render error diagnostics below
        let error_count = self
            .node
            .diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .count();
        let error_lines = format_diagnostic_lines(&self.node.diagnostics);
        for line in error_lines {
            if y >= inner.y + inner.height {
                break;
            }
            buf.set_line(inner.x, y, &line, inner.width);
            y += 1;
        }

        state.click_regions = self.click_regions(inner, error_count);
    }
}

fn format_diagnostic_lines(diagnostics: &[DiagnosticInfo]) -> Vec<Line<'static>> {
    diagnostics
        .iter()
        .filter(|d| d.severity == DiagnosticSeverity::Error)
        .flat_map(|d| {
            d.message.lines().enumerate().map(|(i, line)| {
                let prefix = if i == 0 { "✗ " } else { "  " };
                Line::from(vec![
                    Span::styled(prefix, Style::new().fg(Red::BRIGHT)),
                    Span::styled(line.to_string(), Style::new().fg(Red::BRIGHT)),
                ])
            })
        })
        .collect()
}
