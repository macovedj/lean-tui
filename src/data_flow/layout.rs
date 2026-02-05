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
    graph::{
        calculate_layout, center_on, clip_to_viewport, render_edges, render_scrollbars,
        scroll_target, DagLayout, LayoutConfig,
    },
    protocol::{AnnotatedTextTree, NodeId},
    shared::{
        build_navigation_regions, theme::Theme, ClickRegion, LayoutSize, NavigationRegion,
        Selection,
    },
};

/// State for the functional tableau layout.
#[derive(Default)]
pub struct FunctionalTableauState {
    pub given: BindingGivenPaneState,
    pub proof: FunctionalProofPaneState,
    pub theorem: TheoremPaneState,
}

impl FunctionalTableauState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        self.given
            .find_click_at(x, y)
            .or_else(|| self.proof.find_click_at(x, y))
            .or_else(|| self.theorem.find_click_at(x, y))
    }

    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        self.proof.update_current_node(current_node_id);
    }

    /// Get all navigation regions including initial context bindings and
    /// theorem.
    pub fn navigation_regions(&self) -> Vec<NavigationRegion> {
        let to_nav_region = |cr: &ClickRegion| NavigationRegion {
            x: cr.area.x as i32,
            y: cr.area.y as i32,
            width: cr.area.width,
            height: cr.area.height,
            selection: cr.selection,
        };

        self.given
            .click_regions
            .iter()
            .map(to_nav_region)
            .chain(self.proof.navigation_regions.iter().cloned())
            .chain(self.theorem.click_regions.iter().map(to_nav_region))
            .collect()
    }

    pub fn toggle_stretch(&mut self) {
        self.proof.stretch_children = !self.proof.stretch_children;
    }

    pub const fn stretch_enabled(&self) -> bool {
        self.proof.stretch_children
    }
}

/// State for the functional proof pane.
#[derive(Default)]
pub struct FunctionalProofPaneState {
    scroll_x: i32,
    scroll_y: i32,
    manual_scroll: bool,
    prev_current_node: Option<NodeId>,
    layout: DagLayout,
    pub stretch_children: bool,
    pub click_regions: Vec<ClickRegion>,
    pub navigation_regions: Vec<NavigationRegion>,
}

impl FunctionalProofPaneState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        let point = Rect::new(x, y, 1, 1);
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(point))
            .map(|r| r.selection)
    }

    pub fn update_current_node(&mut self, current_node_id: Option<NodeId>) {
        if self.prev_current_node != current_node_id {
            self.prev_current_node = current_node_id;
            self.manual_scroll = false;
        }
    }
}

/// State for the theorem pane.
#[derive(Default)]
pub struct TheoremPaneState {
    click_regions: Vec<ClickRegion>,
}

impl TheoremPaneState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        let point = Rect::new(x, y, 1, 1);
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(point))
            .map(|r| r.selection)
    }
}

/// Functional tableau layout widget.
pub struct FunctionalTableauLayout<'a> {
    dag: &'a DataFlowDag,
    config: LayoutConfig,
    selection: Option<Selection>,
}

impl<'a> FunctionalTableauLayout<'a> {
    pub const fn new(
        dag: &'a DataFlowDag,
        top_down: bool,
        stretch_children: bool,
        selection: Option<Selection>,
    ) -> Self {
        Self {
            dag,
            config: LayoutConfig {
                top_down,
                stretch_children,
                h_gap: 1,
                v_gap: 1,
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

        BindingGivenPane::new(&self.dag.initial_context, self.selection).render(
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
    state: &mut FunctionalProofPaneState,
) {
    state.click_regions.clear();
    state.navigation_regions.clear();

    if dag.is_empty() || dag.root_node_id.is_none() {
        Paragraph::new("No expression steps")
            .style(Style::new().fg(Color::DarkGray))
            .render(area, buf);
        return;
    }

    let graph = dag.to_graph();

    let layout_config = LayoutConfig {
        top_down: config.top_down,
        stretch_children: state.stretch_children,
        h_gap: config.h_gap,
        v_gap: config.v_gap,
    };

    state.layout = calculate_layout(&graph, layout_config, |node| node.layout_size());
    if state.layout.nodes.is_empty() {
        return;
    }

    state.navigation_regions = build_navigation_regions(dag, &state.layout);

    let selected_idx = selection.and_then(|sel| match sel {
        Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. } => graph.index_of(node_id),
        _ => None,
    });

    let (scroll_x, scroll_y) = if state.manual_scroll {
        (state.scroll_x, state.scroll_y)
    } else {
        scroll_target(&state.layout, selected_idx, graph.current, graph.root)
            .map(|t| {
                center_on(
                    t,
                    state.layout.content_width,
                    state.layout.content_height,
                    area,
                )
            })
            .unwrap_or((0, 0))
    };
    state.scroll_x = scroll_x;
    state.scroll_y = scroll_y;

    render_edges(
        &graph,
        &state.layout,
        scroll_x,
        scroll_y,
        config.top_down,
        area,
        buf,
    );

    let click_regions: Vec<ClickRegion> = state
        .layout
        .nodes
        .iter()
        .filter_map(|pos| {
            let render_area = clip_to_viewport(
                pos.x, pos.y, pos.width, pos.height, scroll_x, scroll_y, area,
            )?;
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

    state.click_regions = click_regions;

    render_scrollbars(
        buf,
        area,
        state.layout.content_width,
        state.layout.content_height,
        scroll_x,
        scroll_y,
    );
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
        .definition_type
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

    state.click_regions.push(ClickRegion {
        area: inner,
        selection: Selection::Theorem,
    });
}
