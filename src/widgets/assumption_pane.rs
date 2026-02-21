use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    selection::{Region, Selection},
    theme::{Blue, Gray, Theme},
    widgets::{
        aligned_table::{AlignedRow, AlignedTable},
        diff_text::RichTextExt,
    },
};

#[derive(Default)]
pub struct AssumptionPaneState {
    click_regions: Vec<Region>,
}

impl AssumptionPaneState {
    pub fn click_regions(&self) -> &[Region] {
        &self.click_regions
    }
}

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

impl StatefulWidget for AssumptionPane<'_> {
    type State = AssumptionPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

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

        let Some(nid) = self.node_id else {
            return;
        };
        state.click_regions.extend(
            visible_hyps
                .iter()
                .enumerate()
                .map(|(row, &hyp_idx)| (inner.y + row as u16, hyp_idx))
                .take_while(|&(row_y, _)| row_y < inner.bottom())
                .map(|(row_y, hyp_idx)| {
                    Region::from_rect(
                        Rect::new(inner.x, row_y, inner.width, 1),
                        Selection::Hyp {
                            node_id: nid,
                            hyp_idx,
                        },
                    )
                }),
        );
    }
}
