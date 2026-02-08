use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    widgets::{Block, Borders, StatefulWidget, Widget},
};

use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    shared::{
        aligned_table::{AlignedRow, AlignedTable},
        diff_text::RichTextExt,
        hypothesis_indices,
        theme::{Blue, Gray, Theme},
        HypothesisFilters, Region, Selection,
    },
};

pub struct AssumptionPaneState {
    click_regions: Vec<Region>,
}

impl Default for AssumptionPaneState {
    fn default() -> Self {
        Self {
            click_regions: Vec::new(),
        }
    }
}

impl AssumptionPaneState {
    pub fn click_regions(&self) -> &[Region] {
        &self.click_regions
    }
}

pub struct AssumptionPane<'a> {
    state: &'a TacticProofState,
    filters: HypothesisFilters,
    selection: Option<Selection>,
    is_focused: bool,
    is_current: bool,
    node_id: Option<NodeId>,
}

impl<'a> AssumptionPane<'a> {
    pub const fn new(
        state: &'a TacticProofState,
        filters: HypothesisFilters,
        selection: Option<Selection>,
        is_focused: bool,
        is_current: bool,
        node_id: Option<NodeId>,
    ) -> Self {
        Self {
            state,
            filters,
            selection,
            is_focused,
            is_current,
            node_id,
        }
    }

    fn should_show_hypothesis(&self, idx: usize) -> bool {
        let Some(h) = self.state.hypotheses.get(idx) else {
            return false;
        };
        if self.filters.hide_instances && h.is_typeclass_instance {
            return false;
        }
        if self.filters.hide_inaccessible && h.is_proof_term {
            return false;
        }
        true
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

        let visible_hyps: Vec<_> =
            hypothesis_indices(self.state.hypotheses.len(), self.filters.reverse_order)
                .filter(|&i| self.should_show_hypothesis(i))
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

        if let Some(nid) = self.node_id {
            for (row, &hyp_idx) in visible_hyps.iter().enumerate() {
                let row_y = inner.y + row as u16;
                if row_y >= inner.bottom() {
                    break;
                }
                state.click_regions.push(Region::from_rect(
                    Rect::new(inner.x, row_y, inner.width, 1),
                    Selection::Hyp {
                        node_id: nid,
                        hyp_idx,
                    },
                ));
            }
        }
    }
}
