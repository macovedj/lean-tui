use std::iter::once;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use super::types::{DataFlowKind, DataFlowNode, SimpleBinding};
use crate::{
    protocol::{BindingKind, DiagnosticSeverity},
    shared::{
        aligned_table::{AlignedRow, AlignedTable},
        theme::{Red, Theme},
        Region, Selection,
    },
};

/// State for a single functional state node widget.
#[derive(Default)]
pub struct FunctionalStateNodeState {
    pub click_regions: Vec<Region>,
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
        // Error nodes get red border regardless of other state
        if self.node.has_error() {
            return Red::BRIGHT;
        }

        match (
            self.is_current,
            self.node.is_leaf(),
            self.node.is_complete(),
        ) {
            (true, _, _) => Theme::CURRENT_NODE_BORDER,
            (_, true, false) => Theme::INCOMPLETE_NODE_BORDER,
            (_, true, true) => Theme::COMPLETED_NODE_BORDER,
            _ => Theme::INACTIVE_NODE_BORDER,
        }
    }

    fn node_kind(&self) -> DataFlowKind {
        self.node.data_flow_kind()
    }

    fn title(&self) -> String {
        let kind = self.node_kind();
        let icon = kind.icon();
        let error_prefix = if self.node.has_error() { "⚠ " } else { "" };
        // Use branch label if available, otherwise use expression content
        let expr_text = self
            .node
            .branch_label()
            .unwrap_or_else(|| self.node.content.to_plain_text());
        match self.node.edges.len() {
            n if n > 1 => format!(" {error_prefix}{icon} {expr_text} [{n}→] "),
            _ => format!(" {error_prefix}{icon} {expr_text} "),
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

    fn bindings_rows(&self) -> Vec<AlignedRow> {
        let bindings = self.node.bindings_after();
        bindings
            .iter()
            .enumerate()
            .map(|(idx, b)| {
                let is_new = self.node.is_new_binding(idx);
                let is_selected = self.is_binding_selected(idx);
                let style = binding_style(b, is_new, is_selected);

                AlignedRow::new(
                    b.name.clone(),
                    style,
                    ":",
                    Style::new().fg(Color::DarkGray),
                    b.type_str.clone(),
                    style,
                )
            })
            .collect()
    }

    fn type_row(&self) -> AlignedRow {
        let underline = if self.is_goal_selected() {
            Modifier::UNDERLINED
        } else {
            Modifier::empty()
        };

        match (self.node.expected_type(), self.node.is_complete()) {
            (Some(ty), _) => AlignedRow::new(
                "",
                Style::default(),
                "⊢",
                Style::new().fg(Color::DarkGray),
                ty,
                Style::new().fg(Theme::GOAL).add_modifier(underline),
            ),
            (None, true) => AlignedRow::new(
                "",
                Style::default(),
                "✓",
                Style::new()
                    .fg(Theme::COMPLETED_GOAL_FG)
                    .add_modifier(Modifier::BOLD),
                "Complete",
                Style::new()
                    .fg(Theme::COMPLETED_GOAL_FG)
                    .add_modifier(Modifier::BOLD),
            ),
            (None, false) => AlignedRow::new(
                "",
                Style::default(),
                "⊢",
                Style::new().fg(Color::DarkGray),
                "...",
                Style::new().fg(Theme::GOAL).add_modifier(underline),
            ),
        }
    }

    fn build_error_lines(&self) -> Vec<String> {
        self.node
            .diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .map(|d| {
                // Truncate message to first line for compact display
                d.message.lines().next().unwrap_or(&d.message).to_string()
            })
            .collect()
    }

    fn click_regions(&self, inner: Rect, table_height: u16, error_count: usize) -> Vec<Region> {
        let bindings = self.node.bindings_after();
        let num_bindings = bindings.len() as u16;

        let (type_y, bindings_start_y) = if self.top_down {
            (inner.y + num_bindings, inner.y)
        } else {
            (inner.y, inner.y + 1)
        };

        let goal_region = Region::from_rect(
            Rect::new(inner.x, type_y, inner.width, 1),
            Selection::Goal {
                node_id: self.node.id,
                goal_idx: 0,
            },
        );

        let binding_regions: Vec<Region> = bindings
            .iter()
            .enumerate()
            .map(|(idx, b)| {
                let binding_width = (b.name.len() + b.type_str.len() + 2) as u16; // "name: type"
                Region::from_rect(
                    Rect::new(
                        inner.x,
                        bindings_start_y + idx as u16,
                        binding_width.min(inner.width),
                        1,
                    ),
                    Selection::Hyp {
                        node_id: self.node.id,
                        hyp_idx: idx,
                    },
                )
            })
            .collect();

        // Error click regions (below the table)
        let error_y = inner.y + table_height;
        let error_regions: Vec<Region> = (0..error_count)
            .map(|error_idx| {
                Region::from_rect(
                    Rect::new(inner.x, error_y + error_idx as u16, inner.width, 1),
                    Selection::Error {
                        node_id: self.node.id,
                        error_idx,
                    },
                )
            })
            .collect();

        once(goal_region)
            .chain(binding_regions)
            .chain(error_regions)
            .collect()
    }
}

impl StatefulWidget for FunctionalStateNode<'_> {
    type State = FunctionalStateNodeState;

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

        let title_style = Style::new()
            .fg(if self.node.has_error() {
                Red::BRIGHT
            } else if self.is_current {
                Theme::CURRENT_NODE_BORDER // White for active node
            } else {
                Color::Gray
            })
            .add_modifier(if self.is_current || self.node.has_error() {
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

        let bindings_rows = self.bindings_rows();
        let type_row = self.type_row();

        let rows: Vec<AlignedRow> = if self.top_down {
            bindings_rows.into_iter().chain(once(type_row)).collect()
        } else {
            once(type_row).chain(bindings_rows).collect()
        };

        let table_height = rows.len() as u16;
        let table_area = Rect::new(
            inner.x,
            inner.y,
            inner.width,
            table_height.min(inner.height),
        );

        let table = AlignedTable::new(rows);
        Widget::render(table, table_area, buf);

        let error_lines = self.build_error_lines();
        let error_count = error_lines.len();
        let error_style = Style::new().fg(Red::BRIGHT);
        let error_y = inner.y + table_height;
        for (i, msg) in error_lines.into_iter().enumerate() {
            let row_y = error_y + i as u16;
            if row_y >= inner.y + inner.height {
                break;
            }
            buf.set_line(
                inner.x,
                row_y,
                &Line::from(Span::styled(msg, error_style)),
                inner.width,
            );
        }

        state.click_regions = self.click_regions(inner, table_height, error_count);
    }
}

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

fn binding_colors(_kind: BindingKind, is_new: bool) -> (Color, Color) {
    if is_new {
        // New bindings: bright blue with background (same as proof hypotheses)
        (Theme::DATA_HYP_FG, Theme::DATA_HYP_BG)
    } else {
        // Existing bindings: dimmer blue, no background
        (Color::Rgb(100, 130, 170), Color::Reset)
    }
}
