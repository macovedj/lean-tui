use std::mem;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::selection::{Region, Selection};

use super::aligned_table::{AlignedRow, AlignedTable};

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

pub struct NodeStyle {
    pub border_color: Color,
    pub border_bold: bool,
    pub title_color: Color,
    pub title_bold: bool,
}

pub struct NodeContent {
    pub title: String,
    pub items: Vec<ContentItem>,
    pub errors: Vec<ErrorLine>,
}

impl NodeContent {
    pub fn layout_size(&self) -> (u16, u16) {
        let title_width = self.title.chars().count();
        let content_widths = collect_content_widths(&self.items, &self.errors);
        let max_content_width = content_widths.into_iter().max().unwrap_or(0);

        let width = (title_width.max(max_content_width) + BORDER_PADDING) as u16;
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

#[derive(Default)]
pub struct NodeWidgetState {
    pub click_regions: Vec<Region>,
}

pub struct NodeContentWidget {
    pub content: NodeContent,
    pub style: NodeStyle,
}

impl StatefulWidget for NodeContentWidget {
    type State = NodeWidgetState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let border_mod = if self.style.border_bold { Modifier::BOLD } else { Modifier::empty() };
        let title_mod = if self.style.title_bold { Modifier::BOLD } else { Modifier::empty() };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(self.style.border_color).add_modifier(border_mod))
            .title(Span::styled(
                self.content.title,
                Style::new().fg(self.style.title_color).add_modifier(title_mod),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.height < 1 {
            return;
        }

        let y = render_items(self.content.items, inner, inner.y, buf, &mut state.click_regions);
        render_errors(self.content.errors, inner, y, buf, &mut state.click_regions);
    }
}

fn render_items(
    items: Vec<ContentItem>,
    inner: Rect,
    start_y: u16,
    buf: &mut Buffer,
    regions: &mut Vec<Region>,
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
                push_region(regions, inner, y, selection);
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
    regions: &mut Vec<Region>,
) {
    let bottom = inner.y + inner.height;
    let mut y = start_y;
    for err in errors {
        if y >= bottom {
            break;
        }
        buf.set_line(inner.x, y, &Line::from(Span::styled(err.text, err.style)), inner.width);
        push_region(regions, inner, y, err.selection);
        y += 1;
    }
}

fn push_region(regions: &mut Vec<Region>, inner: Rect, y: u16, selection: Option<Selection>) {
    if let Some(sel) = selection {
        regions.push(Region::from_rect(Rect::new(inner.x, y, inner.width, 1), sel));
    }
}

fn flush_render_batch(
    rows: &mut Vec<AlignedRow>,
    selections: &mut Vec<Option<Selection>>,
    inner: Rect,
    start_y: u16,
    buf: &mut Buffer,
    regions: &mut Vec<Region>,
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
        push_region(regions, inner, row_y, sel);
    }

    let table_area = Rect::new(inner.x, start_y, inner.width, render_height);
    Widget::render(AlignedTable::new(batch), table_area, buf);
    start_y + render_height
}
