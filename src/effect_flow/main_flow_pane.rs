//! Main flow pane - displays the sequential chain of operations.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::Span,
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

use super::types::{EffectFlowDag, MainFlow};
use crate::{
    protocol::NodeId,
    shared::{ClickRegion, NavigationRegion, Selection, Theme},
};

/// State for the main flow pane.
#[derive(Default)]
pub struct MainFlowPaneState {
    scroll_offset: usize,
    pub click_regions: Vec<ClickRegion>,
    pub navigation_regions: Vec<NavigationRegion>,
}

impl MainFlowPaneState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        let point = Rect::new(x, y, 1, 1);
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(point))
            .map(|r| r.selection)
    }

    pub fn navigation_regions(&self) -> &[NavigationRegion] {
        &self.navigation_regions
    }
}

/// Main flow pane widget showing the sequential chain of operations.
pub struct MainFlowPane<'a> {
    dag: &'a EffectFlowDag,
    main_flow: &'a MainFlow,
    selection: Option<Selection>,
}

impl<'a> MainFlowPane<'a> {
    pub fn new(
        dag: &'a EffectFlowDag,
        main_flow: &'a MainFlow,
        selection: Option<Selection>,
    ) -> Self {
        Self {
            dag,
            main_flow,
            selection,
        }
    }

    fn selected_node_id(&self) -> Option<NodeId> {
        match self.selection {
            Some(Selection::Goal { node_id, .. }) => Some(node_id),
            _ => None,
        }
    }
}

impl StatefulWidget for MainFlowPane<'_> {
    type State = MainFlowPaneState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();
        state.navigation_regions.clear();

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Theme::BORDER))
            .title(Span::styled(
                " Main Flow ",
                Style::new()
                    .fg(Theme::TITLE_GOAL)
                    .add_modifier(Modifier::BOLD),
            ));

        let inner = block.inner(area);
        block.render(area, buf);

        if inner.width < 3 || inner.height < 1 {
            return;
        }

        let selected_id = self.selected_node_id();
        let current_id = self.dag.current_node_id;

        // Auto-scroll to keep selection visible
        let visible_lines = inner.height as usize;
        if let Some(idx) = selected_id.and_then(|id| self.main_flow.index_of(id)) {
            state.scroll_offset = clamp_scroll(idx, state.scroll_offset, visible_lines);
        }

        let visible_start = state.scroll_offset;
        let visible_end = (state.scroll_offset + inner.height as usize).min(self.main_flow.len());

        for (line_idx, (_flow_idx, node_id)) in self
            .main_flow
            .iter_enumerated()
            .skip(visible_start)
            .take(visible_end - visible_start)
            .enumerate()
        {
            let Some(node) = self.dag.get(node_id) else {
                continue;
            };

            let y = inner.y + line_idx as u16;
            let line_area = Rect::new(inner.x, y, inner.width, 1);

            let is_selected = selected_id == Some(node_id);
            let is_current = current_id == Some(node_id);
            let has_details = !self.dag.detail_children(node_id).is_empty();

            // Build the line content
            let icon = node.effect_kind.icon();
            let label = node.content.to_plain_text();
            let detail_indicator = if has_details { " ▸" } else { "" };

            let mut style = Style::new().fg(node.effect_kind.color());
            if is_selected {
                style = style.bg(Theme::SELECTION_BG).add_modifier(Modifier::BOLD);
            }
            if is_current {
                style = style.add_modifier(Modifier::UNDERLINED);
            }

            let content = format!("{icon} {label}{detail_indicator}");
            Paragraph::new(content).style(style).render(line_area, buf);

            // Register click region
            state.click_regions.push(ClickRegion {
                area: line_area,
                selection: Selection::Goal {
                    node_id,
                    goal_idx: 0,
                },
            });

            // Register navigation region
            state.navigation_regions.push(NavigationRegion {
                x: i32::from(line_area.x),
                y: i32::from(line_area.y),
                width: line_area.width,
                height: line_area.height,
                selection: Selection::Goal {
                    node_id,
                    goal_idx: 0,
                },
            });
        }

        // Render scrollbar if needed
        if self.main_flow.len() > inner.height as usize {
            let sb_area = Rect::new(
                area.x + area.width.saturating_sub(1),
                inner.y,
                1,
                inner.height,
            );
            let mut sb_state = ScrollbarState::new(self.main_flow.len())
                .position(state.scroll_offset)
                .viewport_content_length(inner.height as usize);
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("▲"))
                .end_symbol(Some("▼"))
                .render(sb_area, buf, &mut sb_state);
        }
    }
}

/// Adjust scroll offset to ensure the given index is visible.
fn clamp_scroll(idx: usize, current_offset: usize, visible_lines: usize) -> usize {
    if idx < current_offset {
        idx
    } else if idx >= current_offset + visible_lines {
        idx.saturating_sub(visible_lines - 1)
    } else {
        current_offset
    }
}
