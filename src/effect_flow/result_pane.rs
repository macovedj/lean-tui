//! Result type pane for effect flow.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, Widget},
};

use crate::{
    protocol::GenericDag,
    shared::{theme::Theme, truncate_str, ClickRegion, Selection},
};

/// State for the result type pane.
#[derive(Default)]
pub struct ResultPaneState {
    pub click_regions: Vec<ClickRegion>,
}

impl ResultPaneState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        let point = Rect::new(x, y, 1, 1);
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(point))
            .map(|r| r.selection)
    }
}

/// Render the result type pane.
pub fn render_result_pane(
    dag: &GenericDag,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut ResultPaneState,
) {
    state.click_regions.clear();

    let is_selected = matches!(selection, Some(Selection::Theorem));
    let border_style = Style::new().fg(if is_selected {
        Theme::BORDER_FOCUSED
    } else {
        Theme::BORDER
    });

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            " Result Type ",
            Style::new().fg(Theme::TITLE_GOAL),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    state.click_regions.push(ClickRegion {
        area: inner,
        selection: Selection::Theorem,
    });

    let type_text = dag.definition_type.as_ref().map_or_else(
        || "?".to_string(),
        |t| truncate_str(&t.to_plain_text(), inner.width.saturating_sub(2) as usize),
    );

    let style = Style::new()
        .fg(Theme::GOAL_TYPE)
        .add_modifier(if is_selected {
            Modifier::UNDERLINED
        } else {
            Modifier::empty()
        });

    Paragraph::new(Span::styled(type_text, style)).render(inner, buf);
}
