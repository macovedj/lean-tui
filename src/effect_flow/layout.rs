//! Effect flow tableau layout - coordinates the panes.
//!
//! Lay out structure:
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Given: parameters                                           │
//! ├──────────────┬─────────────────┬────────────────────────────┤
//! │ Main Flow    │ Interpretation  │ Detail View               │
//! │              │                 │                           │
//! │ ⊛ deposit    │ → produces x    │  ◁ get                    │
//! │      ↓       │ ← receives y    │    └─ λs. (s, s)          │
//! │ ⊛ withdraw   │                 │  ...                      │
//! │ ...          │                 │                           │
//! ├──────────────┴─────────────────┴────────────────────────────┤
//! │ Result Type: ⊢ BankM Int                                    │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! The detail pane always shows the unfolded internals of the current node
//! (determined by editor cursor position via `current_node_id`).

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};
use tracing::debug;

use super::{
    graph_pane::{render_graph_pane, GraphPaneState},
    main_flow_pane::{MainFlowPane, MainFlowPaneState},
    types::{EffectFlowDag, MainFlow},
};
use crate::{
    data_flow::given_pane::{BindingGivenPane, BindingGivenPaneState},
    protocol::NodeId,
    shared::{theme::Theme, ClickRegion, LayoutConfig, NavigationRegion, Selection},
};

/// State for the result pane.
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

/// State for the effect flow tableau layout.
#[derive(Default)]
pub struct EffectTableauState {
    pub given: BindingGivenPaneState,
    pub main_flow: MainFlowPaneState,
    pub detail: GraphPaneState,
    pub result: ResultPaneState,
}

impl EffectTableauState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        self.given
            .find_click_at(x, y)
            .or_else(|| self.result.find_click_at(x, y))
            .or_else(|| self.main_flow.find_click_at(x, y))
            .or_else(|| self.detail.find_click_at(x, y))
    }

    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        self.detail.update_current_node(current_node_id);
    }

    pub fn navigation_regions(&self) -> Vec<NavigationRegion> {
        let mut regions = self.main_flow.navigation_regions().to_vec();
        regions.extend(self.detail.navigation_regions().iter().copied());
        regions
    }
}

/// Computed layout areas.
struct LayoutAreas {
    given: Rect,
    main_flow: Rect,
    interpretation: Rect,
    detail: Rect,
    result: Rect,
}

/// Effect flow tableau layout widget.
pub struct EffectTableauLayout<'a> {
    dag: &'a EffectFlowDag,
    selection: Option<Selection>,
}

impl<'a> EffectTableauLayout<'a> {
    pub fn new(dag: &'a EffectFlowDag, selection: Option<Selection>) -> Self {
        Self { dag, selection }
    }

    /// Calculate the width needed for the main flow pane based on content.
    fn main_flow_width(&self, main_flow: &MainFlow) -> u16 {
        let content_width = main_flow
            .node_ids()
            .iter()
            .filter_map(|&id| self.dag.get(id))
            .map(|node| {
                let icon_width = node.effect_kind.icon().chars().count();
                let label_width = node.content.to_plain_text().chars().count();
                let has_details = !self.dag.detail_children(node.id).is_empty();
                let detail_width = if has_details { 2 } else { 0 };
                icon_width + 1 + label_width + detail_width
            })
            .max()
            .unwrap_or(10);

        // Add border (2) + padding (1) + scrollbar space (1)
        let total = content_width + 4;
        // Clamp between reasonable bounds
        (total as u16).clamp(15, 40)
    }

    /// Calculate the width needed for the interpretation pane based on content.
    fn interpretation_width(&self, main_flow: &MainFlow) -> u16 {
        let content_width = main_flow
            .node_ids()
            .iter()
            .filter_map(|&id| self.dag.get(id))
            .map(|node| {
                // Format: "→ " (2 chars) + text
                let forward_width = node.forward.as_ref().map_or(0, |s| s.chars().count() + 2);
                let backward_width = node.backward.as_ref().map_or(0, |s| s.chars().count() + 2);
                forward_width.max(backward_width)
            })
            .max()
            .unwrap_or(15);

        // Add border (2) + padding (1)
        let total = content_width + 3;
        // Clamp between reasonable bounds
        (total as u16).clamp(18, 50)
    }

    /// Compute areas for the layout.
    fn compute_layout(&self, area: Rect, main_flow: &MainFlow) -> LayoutAreas {
        let has_result = self.dag.definition_type.is_some();

        // Vertical layout: Given | Main Content | Result
        // Given pane is always visible (shows "No parameters" when empty)
        let [given_area, content_area, result_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(if has_result { 3 } else { 0 }),
        ])
        .areas(area);

        let main_flow_width = self.main_flow_width(main_flow);
        let interp_width = self.interpretation_width(main_flow);

        // Horizontal split: Main Flow (dynamic) | Interpretation (dynamic) | Detail
        // (fill)
        let [main_flow_area, interp_area, detail_area] = Layout::horizontal([
            Constraint::Length(main_flow_width),
            Constraint::Length(interp_width),
            Constraint::Fill(1),
        ])
        .areas(content_area);

        LayoutAreas {
            given: given_area,
            main_flow: main_flow_area,
            interpretation: interp_area,
            detail: detail_area,
            result: result_area,
        }
    }
}

impl StatefulWidget for EffectTableauLayout<'_> {
    type State = EffectTableauState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        // Extract main flow first so we can compute dynamic layout
        let main_flow = self.dag.extract_main_flow();
        let areas = self.compute_layout(area, &main_flow);

        // 1. Given pane (function parameters)
        if areas.given.height > 0 {
            BindingGivenPane::new(&self.dag.initial_context, self.selection).render(
                areas.given,
                buf,
                &mut state.given,
            );
        }

        // 2. Main flow pane (left)
        MainFlowPane::new(self.dag, &main_flow, self.selection).render(
            areas.main_flow,
            buf,
            &mut state.main_flow,
        );

        // 3. Interpretation pane (middle) - shows forward/backward for selected node
        render_interpretation_column(
            self.dag,
            &main_flow,
            self.selection,
            areas.interpretation,
            buf,
        );

        // 4. Detail pane (right) - shows detail subtree of selected main flow node
        render_detail_pane(
            self.dag,
            &main_flow,
            self.selection,
            areas.detail,
            buf,
            &mut state.detail,
        );

        // 5. Result pane (definition type)
        if areas.result.height > 0 {
            let is_selected = matches!(self.selection, Some(Selection::Theorem));
            render_result_pane(self.dag, is_selected, areas.result, buf, &mut state.result);
        }
    }
}

/// Render the detail pane showing the subtree of the selected main flow node.
fn render_detail_pane(
    dag: &EffectFlowDag,
    main_flow: &MainFlow,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut GraphPaneState,
) {
    // Detail pane follows user selection, falling back to current_node_id from
    // editor
    let selected_node_id = match selection {
        Some(Selection::Goal { node_id, .. }) => Some(node_id),
        _ => None,
    };

    let detail_parent_id = selected_node_id
        .filter(|id| main_flow.contains(*id))
        .or_else(|| dag.current_node_id.filter(|id| main_flow.contains(*id)))
        .or_else(|| main_flow.node_ids().first().copied());

    let Some(parent_id) = detail_parent_id else {
        render_empty_detail_pane(area, buf);
        return;
    };

    let detail_subdag = dag.detail_subdag(parent_id);

    debug!(
        "render_detail_pane: parent_id={}, detail_subdag nodes={}, root={:?}",
        parent_id,
        detail_subdag.dag.nodes.len(),
        detail_subdag.dag.root_node_id
    );

    if detail_subdag.is_empty() {
        debug!("  detail_subdag is empty, rendering empty pane");
        render_empty_detail_pane(area, buf);
        return;
    }

    // Get the parent node for the title
    let title = dag
        .get(parent_id)
        .map(|n| format!(" Detail: {} ", n.content.to_plain_text()))
        .unwrap_or_else(|| " Detail ".to_string());

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Theme::BORDER))
        .title(Span::styled(
            title,
            Style::new()
                .fg(Theme::TITLE_GOAL)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 3 || inner.height < 1 {
        return;
    }

    // Render the detail subtree with top-down layout
    let config = LayoutConfig {
        top_down: true,
        stretch_children: false,
    };
    render_graph_pane(
        &detail_subdag.dag,
        config,
        None, // No selection highlighting in detail pane
        inner,
        buf,
        state,
    );
}

/// Render an empty detail pane with a placeholder message.
fn render_empty_detail_pane(area: Rect, buf: &mut Buffer) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Theme::BORDER))
        .title(Span::styled(
            " Detail ",
            Style::new()
                .fg(Theme::TITLE_GOAL)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width >= 10 && inner.height >= 1 {
        Paragraph::new("No detail to show")
            .style(Style::new().fg(Theme::BORDER))
            .render(inner, buf);
    }
}

/// Render the interpretation column showing forward/backward for the selected
/// node.
fn render_interpretation_column(
    dag: &EffectFlowDag,
    main_flow: &MainFlow,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Theme::BORDER))
        .title(Span::styled(
            " Interpretation ",
            Style::new()
                .fg(Theme::TITLE_GOAL)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 3 || inner.height < 1 {
        return;
    }

    let selected_node_id = match selection {
        Some(Selection::Goal { node_id, .. }) => Some(node_id),
        _ => None,
    };

    let target_node_id = selected_node_id
        .filter(|id| main_flow.contains(*id))
        .or_else(|| dag.current_node_id.filter(|id| main_flow.contains(*id)))
        .or_else(|| main_flow.node_ids().first().copied());

    let Some(node) = target_node_id.and_then(|id| dag.get(id)) else {
        return;
    };

    let mut lines = Vec::new();

    if let Some(forward) = &node.forward {
        lines.push(ratatui::text::Line::from(vec![
            Span::styled("→ ", Style::new().fg(Theme::EFFECT_STATE_WRITE)),
            Span::raw(forward.as_str()),
        ]));
    }

    if let Some(backward) = &node.backward {
        lines.push(ratatui::text::Line::from(vec![
            Span::styled("← ", Style::new().fg(Theme::EFFECT_STATE_READ)),
            Span::raw(backward.as_str()),
        ]));
    }

    if lines.is_empty() {
        Paragraph::new("No interpretation")
            .style(Style::new().fg(Theme::BORDER))
            .render(inner, buf);
    } else {
        ratatui::widgets::Paragraph::new(lines)
            .wrap(ratatui::widgets::Wrap { trim: false })
            .render(inner, buf);
    }
}

/// Render the result pane showing the definition type.
fn render_result_pane(
    dag: &EffectFlowDag,
    is_selected: bool,
    area: Rect,
    buf: &mut Buffer,
    state: &mut ResultPaneState,
) {
    state.click_regions.clear();

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Theme::GOAL_FG))
        .title(Span::styled(
            " Result Type ",
            Style::new().fg(Theme::GOAL_FG).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 2 || inner.height < 1 {
        return;
    }

    let type_text = dag
        .definition_type
        .as_ref()
        .map_or_else(|| "...".to_string(), |t| t.to_plain_text());

    let style = Style::new()
        .fg(Theme::GOAL_FG)
        .add_modifier(if is_selected {
            Modifier::UNDERLINED
        } else {
            Modifier::empty()
        });

    Paragraph::new(format!("⊢ {type_text}"))
        .style(style)
        .render(inner, buf);

    state.click_regions.push(ClickRegion {
        area: inner,
        selection: Selection::Theorem,
    });
}
