use std::iter::once;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use super::types::{DataFlowKind, DataFlowNode};
use crate::{
    protocol::{DiagnosticSeverity, LocalBinding},
    selection::{Region, Selection},
    theme::{Red, Theme},
    widgets::{
        aligned_table::{AlignedRow, AlignedTable},
        diff_text::RichTextExt,
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
        let expr_text = self.node.content.to_plain_text();
        match self.node.edges.len() {
            n if n > 1 => format!(" {error_prefix}{icon} {expr_text} [{n}→] "),
            _ => format!(" {error_prefix}{icon} {expr_text} "),
        }
    }

    fn is_goal_selected(&self) -> bool {
        matches!(
            self.selection,
            Some(Selection::Goal { node_id, goal_idx: 0 }) if node_id == self.node.id
        )
    }

    fn is_hyp_selected(&self, hyp_idx: usize) -> bool {
        matches!(
            self.selection,
            Some(Selection::Hyp { node_id, hyp_idx: hi }) if node_id == self.node.id && hi == hyp_idx
        )
    }

    fn visible_bindings(&self) -> Vec<LocalBinding> {
        self.node
            .bindings()
            .into_iter()
            .filter(|b| !b.is_implicit && !b.is_instance)
            .collect()
    }

    fn bindings_rows(&self) -> Vec<AlignedRow> {
        self.visible_bindings()
            .iter()
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
        let mut regions = Vec::new();
        let binding_count = self.visible_bindings().len();
        let mut current_y = inner.y;

        if self.top_down {
            // Bindings first, then goal
            for hyp_idx in 0..binding_count {
                if current_y < inner.y + inner.height {
                    regions.push(Region::from_rect(
                        Rect::new(inner.x, current_y, inner.width, 1),
                        Selection::Hyp {
                            node_id: self.node.id,
                            hyp_idx,
                        },
                    ));
                }
                current_y += 1;
            }
            if current_y < inner.y + inner.height {
                regions.push(Region::from_rect(
                    Rect::new(inner.x, current_y, inner.width, 1),
                    Selection::Goal {
                        node_id: self.node.id,
                        goal_idx: 0,
                    },
                ));
            }
        } else {
            // Goal first, then bindings
            if current_y < inner.y + inner.height {
                regions.push(Region::from_rect(
                    Rect::new(inner.x, current_y, inner.width, 1),
                    Selection::Goal {
                        node_id: self.node.id,
                        goal_idx: 0,
                    },
                ));
            }
            current_y += 1;
            for hyp_idx in 0..binding_count {
                if current_y < inner.y + inner.height {
                    regions.push(Region::from_rect(
                        Rect::new(inner.x, current_y, inner.width, 1),
                        Selection::Hyp {
                            node_id: self.node.id,
                            hyp_idx,
                        },
                    ));
                }
                current_y += 1;
            }
        }

        // Error regions below the table
        current_y = inner.y + table_height;
        for error_idx in 0..error_count {
            if current_y < inner.y + inner.height {
                regions.push(Region::from_rect(
                    Rect::new(inner.x, current_y, inner.width, 1),
                    Selection::Error {
                        node_id: self.node.id,
                        error_idx,
                    },
                ));
            }
            current_y += 1;
        }

        regions
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
