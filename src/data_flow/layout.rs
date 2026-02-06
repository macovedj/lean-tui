use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use super::{
    given_pane::{BindingGivenPane, BindingGivenPaneState},
    state_node::{FunctionalStateNode, FunctionalStateNodeState},
    types::DataFlowDag,
};
use crate::{
    graph_layout::{calculate_layout, GraphViewport, LayoutConfig},
    protocol::{AnnotatedTextTree, NodeId},
    shared::{build_navigation_regions, find_region_at, theme::Theme, Region, Selection},
};

/// State for the functional tableau layout.
#[derive(Default)]
pub struct FunctionalTableauState {
    pub given: BindingGivenPaneState,
    pub proof: GraphViewport,
    pub theorem: TheoremPaneState,
}

impl FunctionalTableauState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        [
            self.given.click_regions.as_slice(),
            self.proof.click_regions.as_slice(),
            self.theorem.click_regions.as_slice(),
        ]
        .into_iter()
        .find_map(|regions| find_region_at(regions, x, y))
    }

    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        self.proof.update_current_node(current_node_id);
    }

    /// Get all navigation regions including initial context bindings and
    /// theorem.
    pub fn navigation_regions(&self) -> Vec<Region> {
        self.given
            .click_regions
            .iter()
            .copied()
            .chain(self.proof.navigation_regions().iter().copied())
            .chain(self.theorem.click_regions.iter().copied())
            .collect()
    }
}

/// State for the theorem pane.
#[derive(Default)]
pub struct TheoremPaneState {
    pub click_regions: Vec<Region>,
}

/// Functional tableau layout widget.
pub struct FunctionalTableauLayout<'a> {
    dag: &'a DataFlowDag,
    config: LayoutConfig,
    selection: Option<Selection>,
}

impl<'a> FunctionalTableauLayout<'a> {
    pub fn new(dag: &'a DataFlowDag, top_down: bool, selection: Option<Selection>) -> Self {
        Self {
            dag,
            config: LayoutConfig {
                top_down,
                ..Default::default()
            },
            selection,
        }
    }

    fn compute_layout(&self, area: Rect) -> [Rect; 3] {
        let constraints = if self.config.top_down {
            [
                Constraint::Length(3),
                Constraint::Fill(1),
                Constraint::Length(3),
            ]
        } else {
            [
                Constraint::Length(3),
                Constraint::Length(3),
                Constraint::Fill(1),
            ]
        };
        Layout::vertical(constraints).areas(area)
    }

    fn pane_areas(&self, area: Rect) -> (Rect, Rect, Rect) {
        let [given, middle, bottom] = self.compute_layout(area);
        if self.config.top_down {
            (given, middle, bottom)
        } else {
            (given, bottom, middle)
        }
    }
}

impl StatefulWidget for FunctionalTableauLayout<'_> {
    type State = FunctionalTableauState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let (given_area, proof_area, theorem_area) = self.pane_areas(area);

        BindingGivenPane::new(&self.dag.block_input, self.selection).render(
            given_area,
            buf,
            &mut state.given,
        );

        render_proof_pane(
            self.dag,
            self.config,
            self.selection,
            proof_area,
            buf,
            &mut state.proof,
        );

        render_theorem_pane(
            self.dag,
            self.selection,
            theorem_area,
            buf,
            &mut state.theorem,
        );
    }
}

fn render_proof_pane(
    dag: &DataFlowDag,
    config: LayoutConfig,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut GraphViewport,
) {
    state.clear_click_regions();

    if dag.is_empty() || dag.root_node_id.is_none() {
        Paragraph::new("No expression steps")
            .style(Style::new().fg(Color::DarkGray))
            .render(area, buf);
        return;
    }

    let graph = dag.to_graph();

    let layout_changed = state.update_layout_if_needed(dag.len(), || {
        let layout_graph = dag.to_layout_graph();
        calculate_layout(&layout_graph, config)
    });

    if layout_changed {
        state.navigation_regions = build_navigation_regions(dag, &state.layout);
    }

    if state.layout.nodes.is_empty() {
        return;
    }

    let selected_idx = selection.and_then(|sel| match sel {
        Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. } => graph.index_of(node_id),
        _ => None,
    });

    state.compute_scroll(selected_idx, graph.current, graph.root, area);
    state.render_edges(&graph, config, area, buf);

    let click_regions: Vec<_> = state
        .layout
        .nodes
        .iter()
        .filter_map(|pos| {
            let render_area =
                state.clip_node_to_viewport(pos.x, pos.y, pos.width, pos.height, area)?;
            let node = graph.get(pos.index)?;
            (render_area.width >= 3 && render_area.height >= 3).then_some((render_area, node, pos))
        })
        .flat_map(|(render_area, node, pos)| {
            let is_current = graph.is_current(pos.index);
            let mut node_state = FunctionalStateNodeState::default();
            FunctionalStateNode::new(node, is_current, selection, config.top_down).render(
                render_area,
                buf,
                &mut node_state,
            );
            node_state.click_regions
        })
        .collect();
    state.click_regions.extend(click_regions);
    state.render_scrollbars(area, buf);
}

fn render_theorem_pane(
    dag: &DataFlowDag,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut TheoremPaneState,
) {
    state.click_regions.clear();

    let is_selected = matches!(selection, Some(Selection::Theorem));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(Theme::GOAL_FG))
        .title(Span::styled(
            " Output ",
            Style::new().fg(Theme::GOAL_FG).add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    if inner.width < 2 || inner.height < 1 {
        return;
    }

    let type_text = dag
        .block_output
        .as_ref()
        .map_or_else(|| "...".to_string(), AnnotatedTextTree::to_plain_text);

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

    state
        .click_regions
        .push(Region::from_rect(inner, Selection::Theorem));
}
