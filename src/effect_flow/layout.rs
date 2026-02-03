//! Effect flow tableau layout - coordinates the panes.
//!
//! This module provides the main `EffectTableauLayout` widget that
//! combines the given, interpretation, graph, and result panes.

use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use super::{
    graph_pane::{render_graph_pane, GraphPaneState},
    interpretation_pane::render_interpretation_pane,
    types::{EffectFlowDag, EffectFlowNode},
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
    pub graph: GraphPaneState,
    pub result: ResultPaneState,
}

impl EffectTableauState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        self.given
            .find_click_at(x, y)
            .or_else(|| self.result.find_click_at(x, y))
            .or_else(|| self.graph.find_click_at(x, y))
    }

    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        self.graph.update_current_node(current_node_id);
    }

    pub fn navigation_regions(&self) -> &[NavigationRegion] {
        self.graph.navigation_regions()
    }
}

/// Effect flow tableau layout widget.
pub struct EffectTableauLayout<'a> {
    dag: &'a EffectFlowDag,
    config: LayoutConfig,
    selection: Option<Selection>,
    show_interpretations: bool,
    collapsed_nodes: &'a HashSet<NodeId>,
}

impl<'a> EffectTableauLayout<'a> {
    pub fn new(
        dag: &'a EffectFlowDag,
        top_down: bool,
        stretch_children: bool,
        selection: Option<Selection>,
        show_interpretations: bool,
        collapsed_nodes: &'a HashSet<NodeId>,
    ) -> Self {
        Self {
            dag,
            config: LayoutConfig {
                top_down,
                stretch_children,
            },
            selection,
            show_interpretations,
            collapsed_nodes,
        }
    }

    /// Compute areas for the 4-section layout: Given, Interpretation, Graph,
    /// Result.
    fn compute_layout(&self, area: Rect) -> [Rect; 4] {
        let has_given = !self.dag.initial_context.is_empty();
        let has_result = self.dag.definition_type.is_some();
        let interp_height = if self.show_interpretations { 5 } else { 0 };

        Layout::vertical([
            Constraint::Length(if has_given { 3 } else { 0 }), // Given
            Constraint::Length(interp_height),                 // Interpretations
            Constraint::Fill(1),                               // Graph (scrollable)
            Constraint::Length(if has_result { 3 } else { 0 }), // Result
        ])
        .areas(area)
    }

    /// Get the currently selected node for interpretation display.
    fn selected_node(&self) -> Option<&EffectFlowNode> {
        let node_id = match self.selection {
            Some(Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. }) => Some(node_id),
            _ => self.dag.current_node_id,
        }?;
        self.dag.get(node_id)
    }
}

impl StatefulWidget for EffectTableauLayout<'_> {
    type State = EffectTableauState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let [given_area, interp_area, graph_area, result_area] = self.compute_layout(area);

        // 1. Given pane (function parameters)
        if given_area.height > 0 {
            BindingGivenPane::new(&self.dag.initial_context, self.selection).render(
                given_area,
                buf,
                &mut state.given,
            );
        }

        // 2. Interpretations pane
        if self.show_interpretations && interp_area.height > 0 {
            render_interpretation_pane(self.selected_node(), interp_area, buf);
        }

        // 3. Graph pane
        render_graph_pane(
            self.dag,
            self.config,
            self.selection,
            self.collapsed_nodes,
            graph_area,
            buf,
            &mut state.graph,
        );

        // 4. Result pane (definition type)
        if result_area.height > 0 {
            let is_selected = matches!(self.selection, Some(Selection::Theorem));
            render_result_pane(self.dag, is_selected, result_area, buf, &mut state.result);
        }
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
