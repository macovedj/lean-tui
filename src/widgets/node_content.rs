use std::mem;

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Widget},
};

use super::aligned_table::{AlignedRow, AlignedTable};
use crate::{
    message::{selection_click_target, ClickTarget},
    protocol::{format_timing, TimingDisplay},
    selection::Selection,
    theme::{Red, Theme},
};

const BORDER_PADDING: usize = 6;
const BORDER_HEIGHT: u16 = 2;

pub enum ContentItem {
    Row {
        row: AlignedRow,
        selection: Option<Selection>,
    },
    Line {
        spans: Vec<Span<'static>>,
        selection: Option<Selection>,
    },
}

pub struct ErrorLine {
    pub text: String,
    pub style: Style,
    pub selection: Option<Selection>,
}

pub enum NodeVisualState {
    Error,
    Current,
    CompletedLeaf,
    IncompleteLeaf,
    Inactive,
}

pub struct NodeStyle {
    pub border_color: Color,
    pub border_bold: bool,
    pub title_color: Color,
    pub title_bold: bool,
}

impl From<NodeVisualState> for NodeStyle {
    fn from(state: NodeVisualState) -> Self {
        let bold = matches!(state, NodeVisualState::Error | NodeVisualState::Current);
        let border_color = match state {
            NodeVisualState::Error => Red::BRIGHT,
            NodeVisualState::Current => Theme::CURRENT_NODE_BORDER,
            NodeVisualState::CompletedLeaf => Theme::COMPLETED_NODE_BORDER,
            NodeVisualState::IncompleteLeaf => Theme::INCOMPLETE_NODE_BORDER,
            NodeVisualState::Inactive => Theme::INACTIVE_NODE_BORDER,
        };
        let title_color = match state {
            NodeVisualState::Error => Red::BRIGHT,
            NodeVisualState::Current => Theme::CURRENT_NODE_BORDER,
            _ => Color::Gray,
        };
        Self {
            border_color,
            border_bold: bold,
            title_color,
            title_bold: bold,
        }
    }
}

pub struct NodeContent {
    pub title: String,
    pub items: Vec<ContentItem>,
    pub errors: Vec<ErrorLine>,
    pub self_elaboration_ns: Option<u64>,
    pub self_elaboration_heartbeats: Option<u64>,
    pub timing_display: TimingDisplay,
}

impl NodeContent {
    pub fn layout_size(&self) -> (u16, u16) {
        let title_width = self.title.chars().count();
        let elapsed_text = format_timing(
            self.self_elaboration_ns,
            self.self_elaboration_heartbeats,
            self.timing_display,
        );
        let elapsed_width = if elapsed_text.is_empty() {
            0
        } else {
            elapsed_text.len() + 2
        };
        let content_widths = collect_content_widths(&self.items, &self.errors);
        let max_content_width = content_widths.into_iter().max().unwrap_or(0);

        let width = (title_width.max(max_content_width).max(elapsed_width) + BORDER_PADDING) as u16;
        let height = BORDER_HEIGHT + self.items.len() as u16 + self.errors.len() as u16;

        (width, height.max(BORDER_HEIGHT + 1))
    }
}

fn collect_content_widths(items: &[ContentItem], errors: &[ErrorLine]) -> Vec<usize> {
    let mut widths: Vec<usize> = Vec::new();
    let mut row_group: Vec<&AlignedRow> = Vec::new();

    for item in items {
        match item {
            ContentItem::Row { row, .. } => row_group.push(row),
            ContentItem::Line { spans, .. } => {
                flush_row_group(&mut row_group, &mut widths);
                let line_width: usize = spans.iter().map(|s| s.content.chars().count()).sum();
                widths.push(line_width);
            }
        }
    }
    flush_row_group(&mut row_group, &mut widths);

    widths.extend(errors.iter().map(|e| e.text.chars().count()));
    widths
}

fn flush_row_group(row_group: &mut Vec<&AlignedRow>, widths: &mut Vec<usize>) {
    if row_group.is_empty() {
        return;
    }
    widths.push(aligned_table_width(row_group));
    row_group.clear();
}

fn aligned_table_width(rows: &[&AlignedRow]) -> usize {
    let max_name = rows.iter().map(|r| r.name_width()).max().unwrap_or(0);
    let max_type = rows.iter().map(|r| r.type_width()).max().unwrap_or(0);
    max_name + 1 + max_type
}

pub struct NodeContentWidget {
    pub content: NodeContent,
    pub style: NodeStyle,
}

impl NodeContentWidget {
    pub fn draw(self, area: Rect, buf: &mut Buffer) -> Vec<ClickTarget> {
        let border_mod = if self.style.border_bold {
            Modifier::BOLD
        } else {
            Modifier::empty()
        };
        let title_mod = if self.style.title_bold {
            Modifier::BOLD
        } else {
            Modifier::empty()
        };

        let elapsed = format_timing(
            self.content.self_elaboration_ns,
            self.content.self_elaboration_heartbeats,
            self.content.timing_display,
        );
        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(
                Style::new()
                    .fg(self.style.border_color)
                    .add_modifier(border_mod),
            )
            .title(Span::styled(
                self.content.title,
                Style::new()
                    .fg(self.style.title_color)
                    .add_modifier(title_mod),
            ));
        if !elapsed.is_empty() {
            block = block.title_bottom(
                Line::from(Span::styled(
                    format!(" {elapsed} "),
                    Style::new().fg(Color::DarkGray),
                ))
                .alignment(Alignment::Right),
            );
        }

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return Vec::new();
        }

        let mut click_targets = Vec::new();
        let y = render_items(self.content.items, inner, inner.y, buf, &mut click_targets);
        render_errors(self.content.errors, inner, y, buf, &mut click_targets);
        click_targets
    }
}

fn render_items(
    items: Vec<ContentItem>,
    inner: Rect,
    start_y: u16,
    buf: &mut Buffer,
    regions: &mut Vec<ClickTarget>,
) -> u16 {
    let mut y = start_y;
    let mut row_batch: Vec<AlignedRow> = Vec::new();
    let mut row_selections: Vec<Option<Selection>> = Vec::new();
    let bottom = inner.y + inner.height;

    for item in items {
        match item {
            ContentItem::Row { row, selection } => {
                row_batch.push(row);
                row_selections.push(selection);
            }
            ContentItem::Line { spans, selection } => {
                y = flush_render_batch(&mut row_batch, &mut row_selections, inner, y, buf, regions);
                if y >= bottom {
                    return y;
                }
                buf.set_line(inner.x, y, &Line::from(spans), inner.width);
                regions.extend(selection.map(|selection| row_click_target(inner, y, selection)));
                y += 1;
            }
        }
    }
    flush_render_batch(&mut row_batch, &mut row_selections, inner, y, buf, regions)
}

fn render_errors(
    errors: Vec<ErrorLine>,
    inner: Rect,
    start_y: u16,
    buf: &mut Buffer,
    regions: &mut Vec<ClickTarget>,
) {
    let bottom = inner.y + inner.height;
    let mut y = start_y;
    for err in errors {
        if y >= bottom {
            break;
        }
        buf.set_line(
            inner.x,
            y,
            &Line::from(Span::styled(err.text, err.style)),
            inner.width,
        );
        regions.extend(
            err.selection
                .map(|selection| row_click_target(inner, y, selection)),
        );
        y += 1;
    }
}

fn row_click_target(inner: Rect, y: u16, selection: Selection) -> ClickTarget {
    selection_click_target(Rect::new(inner.x, y, inner.width, 1), selection)
}

fn flush_render_batch(
    rows: &mut Vec<AlignedRow>,
    selections: &mut Vec<Option<Selection>>,
    inner: Rect,
    start_y: u16,
    buf: &mut Buffer,
    regions: &mut Vec<ClickTarget>,
) -> u16 {
    if rows.is_empty() {
        return start_y;
    }

    let batch = mem::take(rows);
    let sels = mem::take(selections);
    let row_count = batch.len() as u16;
    let available = inner.y + inner.height - start_y;
    let render_height = row_count.min(available);

    for (i, sel) in sels.into_iter().enumerate() {
        let row_y = start_y + i as u16;
        if row_y >= inner.y + inner.height {
            break;
        }
        regions.extend(sel.map(|selection| row_click_target(inner, row_y, selection)));
    }

    let table_area = Rect::new(inner.x, start_y, inner.width, render_height);
    Widget::render(AlignedTable::new(batch), table_area, buf);
    start_y + render_height
}
