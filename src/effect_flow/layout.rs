//! Effect flow tableau layout - coordinates the panes.
//!
//! This module provides the main `EffectTableauLayout` widget that
//! combines the given, interpretation, graph, and result panes.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::StatefulWidget,
};

use super::{
    graph_pane::{render_graph_pane, GraphPaneState},
    interpretation_pane::render_interpretation_pane,
    result_pane::{render_result_pane, ResultPaneState},
};
use crate::{
    data_flow::given_pane::{BindingGivenPane, BindingGivenPaneState},
    protocol::{GenericDag, GraphNode, NodeId},
    shared::{LayoutConfig, NavigationRegion, Selection},
};

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
            .or_else(|| self.graph.find_click_at(x, y))
            .or_else(|| self.result.find_click_at(x, y))
    }

    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        self.graph.update_current_node(current_node_id);
    }

    pub fn navigation_regions(&self) -> &[NavigationRegion] {
        self.graph.navigation_regions()
    }

    /// Toggle stretch mode for children layout.
    pub fn toggle_stretch(&mut self) {
        self.graph.toggle_stretch();
    }

    /// Check if stretch mode is enabled.
    pub fn stretch_enabled(&self) -> bool {
        self.graph.stretch_enabled()
    }
}

/// Effect flow tableau layout widget.
pub struct EffectTableauLayout<'a> {
    dag: &'a GenericDag,
    config: LayoutConfig,
    selection: Option<Selection>,
    show_interpretations: bool,
}

impl<'a> EffectTableauLayout<'a> {
    pub const fn new(
        dag: &'a GenericDag,
        top_down: bool,
        stretch_children: bool,
        selection: Option<Selection>,
        show_interpretations: bool,
    ) -> Self {
        Self {
            dag,
            config: LayoutConfig {
                top_down,
                stretch_children,
            },
            selection,
            show_interpretations,
        }
    }

    /// Compute areas for the 4-section layout: Given, Interpretation, Graph,
    /// Result.
    fn compute_layout(&self, area: Rect) -> [Rect; 4] {
        let interp_height = if self.show_interpretations { 4 } else { 0 };

        if self.config.top_down {
            // Top-down: Given → Interp → Graph → Result
            Layout::vertical([
                Constraint::Length(3),             // Given
                Constraint::Length(interp_height), // Interpretations
                Constraint::Fill(1),               // Graph (scrollable)
                Constraint::Length(3),             // Result
            ])
            .areas(area)
        } else {
            // Bottom-up: Given → Interp → Result → Graph
            Layout::vertical([
                Constraint::Length(3),             // Given
                Constraint::Length(interp_height), // Interpretations
                Constraint::Length(3),             // Result
                Constraint::Fill(1),               // Graph
            ])
            .areas(area)
        }
    }

    /// Get the currently selected node for interpretation display.
    fn selected_node(&self) -> Option<&GraphNode> {
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
        let [given_area, interp_area, middle_area, bottom_area] = self.compute_layout(area);

        // 1. Given pane (function parameters)
        BindingGivenPane::new(&self.dag.initial_context, self.selection).render(
            given_area,
            buf,
            &mut state.given,
        );

        // 2. Interpretation pane (fwd/bwd for selected node)
        if self.show_interpretations && interp_area.height > 0 {
            render_interpretation_pane(self.selected_node(), interp_area, buf);
        }

        // 3. Effect graph pane and Result pane (order depends on top_down)
        let (graph_area, result_area) = if self.config.top_down {
            (middle_area, bottom_area)
        } else {
            (bottom_area, middle_area)
        };

        render_graph_pane(
            self.dag,
            self.config,
            self.selection,
            graph_area,
            buf,
            &mut state.graph,
        );

        // 4. Result type pane
        render_result_pane(
            self.dag,
            self.selection,
            result_area,
            buf,
            &mut state.result,
        );
    }
}
