use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

use super::hyp_layer::{HypLayer, HypLayerRenderContext};
use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    shared::{
        aligned_table::AlignedTable,
        theme::{Blue, Theme},
        HypothesisFilters, Region, Selection,
    },
};

/// State for the hypothesis section widget.
#[derive(Default)]
pub struct HypSectionState {
    layer: HypLayer,
    depends_on: HashSet<String>,
    selection: Option<Selection>,
    click_regions: Vec<Region>,
    scroll_state: ScrollbarState,
    vertical_scroll: usize,
    /// Whether this pane is currently focused.
    is_focused: bool,
}

impl HypSectionState {
    /// Update the state with new data.
    pub fn update(
        &mut self,
        state: &TacticProofState,
        filters: HypothesisFilters,
        depends_on: HashSet<String>,
        selection: Option<Selection>,
        node_id: Option<NodeId>,
    ) {
        self.layer = HypLayer::new();
        self.layer.set_node_id(node_id);
        self.depends_on = depends_on;
        self.selection = selection;
        let mut seen: HashSet<String> = HashSet::new();

        for (hyp_idx, h) in state.hypotheses.iter().enumerate() {
            if filters.hide_instances && h.is_typeclass_instance {
                continue;
            }
            if filters.hide_inaccessible && h.is_proof_term {
                continue;
            }
            if seen.insert(h.name.clone()) {
                self.layer.add_from_info(hyp_idx, h);
            }
        }
    }

    /// Get the click regions computed during the last render.
    pub fn click_regions(&self) -> &[Region] {
        &self.click_regions
    }

    pub const fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }
}

/// Widget for rendering the hypothesis section.
pub struct HypSection;

impl StatefulWidget for HypSection {
    type State = HypSectionState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let border_color = if state.is_focused {
            Blue::BRIGHT
        } else {
            Blue::MUTED
        };
        let title = if state.is_focused {
            "▶ Hypotheses "
        } else {
            " Hypotheses "
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(title)
            .title_style(Style::new().fg(Theme::TITLE_HYPOTHESIS));

        let inner = block.inner(area);
        block.render(area, buf);

        if state.layer.len() == 0 {
            Paragraph::new("(no hypotheses)")
                .style(Theme::DIM.add_modifier(Modifier::ITALIC))
                .render(inner, buf);
            return;
        }

        if let Some(Selection::Hyp { hyp_idx, .. }) = state.selection {
            state.vertical_scroll = hyp_idx;
        }

        let total_hyps = state.layer.len();
        let viewport_height = inner.height as usize;

        state.scroll_state = state
            .scroll_state
            .content_length(total_hyps)
            .position(state.vertical_scroll);

        let render_ctx = HypLayerRenderContext {
            selected: state.selection,
            base_y: inner.y,
            area: inner,
            depends_on: &state.depends_on,
        };
        let rows = state.layer.render(&render_ctx, &mut state.click_regions);
        let table = AlignedTable::new(rows);
        Widget::render(table, inner, buf);

        if total_hyps > viewport_height && inner.width > 1 {
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y + 1, // Skip top border
                width: 1,
                height: area.height.saturating_sub(1),
            };
            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                scrollbar_area,
                buf,
                &mut state.scroll_state,
            );
        }
    }
}
