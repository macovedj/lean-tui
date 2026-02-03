use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget},
};

use super::{
    state_node::{StateNode, StateNodeState},
    types::CompleteProofDag,
};
use crate::{
    protocol::{NodeId, TacticProofState},
    shared::{
        build_navigation_regions, calculate_dag_layout, ClickRegion, DagLayout, LayoutConfig,
        NavigationRegion, NodePosition, Selection, VirtualCanvas,
    },
};

/// State for the proof pane widget.
#[derive(Default)]
pub struct ProofPaneState {
    scroll_x: i32,
    scroll_y: i32,
    manual_scroll: bool,
    prev_current_node: Option<NodeId>,
    layout: DagLayout,
    content_width: i32,
    content_height: i32,
    pub click_regions: Vec<ClickRegion>,
    pub navigation_regions: Vec<NavigationRegion>,
}

impl ProofPaneState {
    pub fn update_current_node(&mut self, current_node: Option<NodeId>) {
        if current_node != self.prev_current_node {
            self.manual_scroll = false;
            self.prev_current_node = current_node;
        }
    }

    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(Rect::new(x, y, 1, 1)))
            .map(|r| r.selection)
    }

    pub fn navigation_regions(&self) -> &[NavigationRegion] {
        &self.navigation_regions
    }
}

/// Proof pane widget.
pub struct ProofPane<'a> {
    dag: &'a CompleteProofDag,
    top_down: bool,
    selection: Option<Selection>,
    current_state: &'a TacticProofState,
}

impl<'a> ProofPane<'a> {
    pub const fn new(
        dag: &'a CompleteProofDag,
        top_down: bool,
        selection: Option<Selection>,
        current_state: &'a TacticProofState,
    ) -> Self {
        Self {
            dag,
            top_down,
            selection,
            current_state,
        }
    }
}

impl StatefulWidget for ProofPane<'_> {
    type State = ProofPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();
        state.navigation_regions.clear();

        if self.dag.is_empty() || self.dag.root_node_id.is_none() {
            Paragraph::new("No proof steps")
                .style(Style::new().fg(Color::DarkGray))
                .render(area, buf);
            return;
        }

        let config = LayoutConfig {
            top_down: self.top_down,
            stretch_children: false, // Semantic tableau doesn't support stretch mode yet
        };
        state.layout = calculate_dag_layout(self.dag, config);
        state.content_width = state.layout.content_width;
        state.content_height = state.layout.content_height;

        if state.layout.nodes.is_empty() {
            return;
        }

        state.navigation_regions = build_navigation_regions(self.dag, &state.layout);

        let canvas = create_canvas(state, self.dag, self.selection, area);
        state.scroll_x = canvas.scroll_x;
        state.scroll_y = canvas.scroll_y;

        for pos in &state.layout.nodes {
            let Some(render_area) =
                canvas.clip_to_viewport(pos.x, pos.y, pos.width, pos.height, area)
            else {
                continue;
            };
            let Some(node) = self.dag.get(pos.node_id) else {
                continue;
            };
            if render_area.width < 3 || render_area.height < 3 {
                continue;
            }

            let is_current = self.dag.is_current(pos.node_id);
            let override_state = if is_current && !self.current_state.goals.is_empty() {
                Some(self.current_state)
            } else {
                None
            };

            let node_widget = StateNode::new(
                node,
                is_current,
                self.selection,
                self.top_down,
                override_state,
            );
            let mut node_state = StateNodeState::default();
            node_widget.render(render_area, buf, &mut node_state);
            state.click_regions.extend(node_state.click_regions);
        }

        render_scrollbars(buf, area, &canvas);
    }
}

fn create_canvas(
    state: &ProofPaneState,
    dag: &CompleteProofDag,
    selection: Option<Selection>,
    area: Rect,
) -> VirtualCanvas {
    let canvas = VirtualCanvas::new(state.content_width, state.content_height);

    if state.manual_scroll {
        canvas.with_scroll(state.scroll_x, state.scroll_y, area)
    } else {
        find_scroll_target(&state.layout, dag, selection).map_or(canvas, |t| {
            canvas.centered_on(t.x, t.y, t.width, t.height, area)
        })
    }
}

fn render_scrollbars(buf: &mut Buffer, area: Rect, canvas: &VirtualCanvas) {
    if canvas.needs_vertical_scroll(area) && area.height > 2 {
        let sb_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y,
            1,
            area.height,
        );
        let mut sb_state = ScrollbarState::new(canvas.content_height.max(0) as usize)
            .position(canvas.scroll_y.max(0) as usize)
            .viewport_content_length(area.height as usize);
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .render(sb_area, buf, &mut sb_state);
    }

    if canvas.needs_horizontal_scroll(area) && area.width > 2 {
        let sb_area = Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width.saturating_sub(1),
            1,
        );
        let mut sb_state = ScrollbarState::new(canvas.content_width.max(0) as usize)
            .position(canvas.scroll_x.max(0) as usize)
            .viewport_content_length(area.width as usize);
        Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .begin_symbol(Some("◀"))
            .end_symbol(Some("▶"))
            .render(sb_area, buf, &mut sb_state);
    }
}

fn find_scroll_target<'a>(
    layout: &'a DagLayout,
    dag: &CompleteProofDag,
    selection: Option<Selection>,
) -> Option<&'a NodePosition> {
    let selected_node_id = selection.and_then(|sel| match sel {
        Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. } => Some(node_id),
        _ => None,
    });

    selected_node_id
        .and_then(|id| layout.find_node(id))
        .or_else(|| dag.current_node_id.and_then(|id| layout.find_node(id)))
        .or_else(|| dag.root_node_id.and_then(|id| layout.find_node(id)))
}
