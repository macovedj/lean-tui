use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, Widget},
};

use crate::{
    deduction_tree::TacticProofState,
    message::{selection_click_target, ClickTarget},
    protocol::NodeId,
    selection::Selection,
    theme::{Blue, Gray, Theme},
    widgets::{
        aligned_table::{AlignedRow, AlignedTable},
        diff_text::RichTextExt,
    },
};

pub struct AssumptionPane<'a> {
    state: &'a TacticProofState,
    selection: Option<Selection>,
    is_focused: bool,
    is_current: bool,
    node_id: Option<NodeId>,
}

impl<'a> AssumptionPane<'a> {
    pub fn new(
        state: &'a TacticProofState,
        selection: Option<Selection>,
        is_focused: bool,
        is_current: bool,
        node_id: Option<NodeId>,
    ) -> Self {
        Self {
            state,
            selection,
            is_focused,
            is_current,
            node_id,
        }
    }
}

impl AssumptionPane<'_> {
    pub fn draw(self, area: Rect, buf: &mut Buffer) -> Vec<ClickTarget> {
        let border_color = if self.is_focused {
            Blue::BRIGHT
        } else if self.is_current {
            Blue::MUTED
        } else {
            Gray::DIM
        };

        let title = if self.is_focused {
            "▶ Assumptions "
        } else {
            " Assumptions "
        };

        let title_color = if self.is_current {
            Theme::TITLE_ASSUMPTION
        } else {
            Gray::DIM
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(title)
            .title_style(Style::new().fg(title_color));

        let inner = block.inner(area);
        block.render(area, buf);

        let visible_hyps: Vec<_> = (0..self.state.hypotheses.len())
            .take(inner.height as usize)
            .collect();

        let rows: Vec<AlignedRow> = visible_hyps
            .iter()
            .map(|&hyp_idx| {
                let h = &self.state.hypotheses[hyp_idx];
                let is_selected = matches!(
                    self.selection,
                    Some(Selection::Hyp { hyp_idx: sel, .. }) if sel == hyp_idx
                );

                let base_style = if is_selected {
                    Style::new().bg(Theme::SELECTION_BG)
                } else {
                    Style::default()
                };

                let name_color = if self.is_current {
                    Theme::HYP_NAME
                } else {
                    Gray::DIM
                };

                AlignedRow::with_styled_type(
                    h.name.clone(),
                    base_style.fg(name_color),
                    ":",
                    Style::new().fg(Gray::DIM),
                    h.type_.to_spans(base_style),
                )
            })
            .collect();

        let table = AlignedTable::new(rows);
        Widget::render(table, inner, buf);

        let Some(node_id) = self.node_id else {
            return Vec::new();
        };
        visible_hyps
            .iter()
            .zip(inner.rows())
            .map(|(&hyp_idx, row_area)| {
                selection_click_target(row_area, Selection::Hyp { node_id, hyp_idx })
            })
            .collect()
    }
}
