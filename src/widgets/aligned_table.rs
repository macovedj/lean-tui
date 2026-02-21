use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Cell, Row, Table, Widget},
};

/// Content for a table cell - either plain text with a style, or styled spans.
pub enum CellContent {
    Plain { text: String, style: Style },
    Styled(Vec<Span<'static>>),
}

impl CellContent {
    pub fn plain(text: impl Into<String>, style: Style) -> Self {
        Self::Plain {
            text: text.into(),
            style,
        }
    }

    pub fn styled(spans: Vec<Span<'static>>) -> Self {
        Self::Styled(spans)
    }

    pub fn width(&self) -> usize {
        match self {
            Self::Plain { text, .. } => text.chars().count(),
            Self::Styled(spans) => spans.iter().map(|s| s.content.chars().count()).sum(),
        }
    }

    fn into_cell(self) -> Cell<'static> {
        match self {
            Self::Plain { text, style } => Cell::from(text).style(style),
            Self::Styled(spans) => Cell::from(Line::from(spans)),
        }
    }
}

/// A single row in an aligned three-column table.
pub struct AlignedRow {
    pub name: String,
    pub name_style: Style,
    pub separator: &'static str,
    pub separator_style: Style,
    pub type_content: CellContent,
}

impl AlignedRow {
    pub fn new(
        name: impl Into<String>,
        name_style: Style,
        separator: &'static str,
        separator_style: Style,
        type_text: impl Into<String>,
        type_style: Style,
    ) -> Self {
        Self {
            name: name.into(),
            name_style,
            separator,
            separator_style,
            type_content: CellContent::plain(type_text, type_style),
        }
    }

    pub fn with_styled_type(
        name: impl Into<String>,
        name_style: Style,
        separator: &'static str,
        separator_style: Style,
        type_spans: Vec<Span<'static>>,
    ) -> Self {
        Self {
            name: name.into(),
            name_style,
            separator,
            separator_style,
            type_content: CellContent::styled(type_spans),
        }
    }

    pub fn name_width(&self) -> usize {
        self.name.chars().count()
    }

    pub fn type_width(&self) -> usize {
        self.type_content.width()
    }

    fn into_row(self) -> Row<'static> {
        Row::new(vec![
            Cell::from(self.name).style(self.name_style),
            Cell::from(self.separator).style(self.separator_style),
            self.type_content.into_cell(),
        ])
    }
}

/// A three-column aligned table widget.
///
/// Renders rows with:
/// - Column 1: Name (fixed width based on max name length)
/// - Column 2: Separator (single character like ":" or "⊢")
/// - Column 3: Type text (fills remaining space)
pub struct AlignedTable {
    rows: Vec<AlignedRow>,
}

impl AlignedTable {
    pub fn new(rows: Vec<AlignedRow>) -> Self {
        Self { rows }
    }

    fn max_name_width(&self) -> u16 {
        self.rows
            .iter()
            .map(AlignedRow::name_width)
            .max()
            .unwrap_or(0) as u16
    }
}

impl Widget for AlignedTable {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let name_width = self.max_name_width().max(1);

        let rows: Vec<Row> = self.rows.into_iter().map(AlignedRow::into_row).collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(name_width),
                Constraint::Length(1),
                Constraint::Fill(1),
            ],
        );

        Widget::render(table, area, buf);
    }
}
