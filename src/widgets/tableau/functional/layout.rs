//! Layout for functional tableau - orchestrates the three panes for functional
//! DAG visualization.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

use super::{
    given_pane::{BindingGivenPane, BindingGivenPaneState},
    navigation::build_navigation_regions,
    state_node::{FunctionalStateNode, FunctionalStateNodeState},
    tree_layout::calculate_tree_layout,
};
use crate::{
    protocol::{AnnotatedTextTree, CompleteFunctionalDag, NodeId},
    widgets::{
        tableau::{truncate_str, NavigationRegion, NodePosition, TreeLayout, VirtualCanvas},
        theme::Theme,
        ClickRegion, Selection,
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

    pub fn navigation_regions(&self) -> &[NavigationRegion] {
        &self.proof.navigation_regions
    }
}

/// State for the functional proof pane.
#[derive(Default)]
pub struct FunctionalProofPaneState {
    scroll_x: i32,
    scroll_y: i32,
    manual_scroll: bool,
    prev_current_node: Option<NodeId>,
    layout: TreeLayout,
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
    dag: &'a CompleteFunctionalDag,
    top_down: bool,
    selection: Option<Selection>,
}

impl<'a> FunctionalTableauLayout<'a> {
    pub const fn new(
        dag: &'a CompleteFunctionalDag,
        top_down: bool,
        selection: Option<Selection>,
    ) -> Self {
        Self {
            dag,
            top_down,
            selection,
        }
    }

    fn compute_layout(&self, area: Rect) -> [Rect; 3] {
        let constraints = if self.top_down {
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
        if self.top_down {
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

        BindingGivenPane::new(&self.dag.initial_bindings, self.selection).render(
            given_area,
            buf,
            &mut state.given,
        );

        render_proof_pane(
            self.dag,
            self.top_down,
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
    dag: &CompleteFunctionalDag,
    top_down: bool,
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

    state.layout = calculate_tree_layout(dag, top_down);
    if state.layout.nodes.is_empty() {
        return;
    }

    state.navigation_regions = build_navigation_regions(dag, &state.layout);

    let canvas = create_canvas(state, dag, selection, area);
    state.scroll_x = canvas.scroll_x;
    state.scroll_y = canvas.scroll_y;

    let click_regions: Vec<ClickRegion> = state
        .layout
        .nodes
        .iter()
        .filter_map(|pos| {
            let render_area = canvas.clip_to_viewport(pos.x, pos.y, pos.width, pos.height, area)?;
            let node = dag.get(pos.node_id)?;
            (render_area.width >= 3 && render_area.height >= 3).then_some((render_area, node, pos))
        })
        .flat_map(|(render_area, node, pos)| {
            let mut node_state = FunctionalStateNodeState::default();
            FunctionalStateNode::new(node, dag.is_current(pos.node_id), selection, top_down)
                .render(render_area, buf, &mut node_state);
            node_state.click_regions
        })
        .collect();

    state.click_regions = click_regions;

    render_scrollbars(buf, area, &canvas);
}

fn create_canvas(
    state: &FunctionalProofPaneState,
    dag: &CompleteFunctionalDag,
    selection: Option<Selection>,
    area: Rect,
) -> VirtualCanvas {
    let canvas = VirtualCanvas::new(state.layout.content_width, state.layout.content_height);

    if state.manual_scroll {
        canvas.with_scroll(state.scroll_x, state.scroll_y, area)
    } else {
        find_scroll_target(&state.layout, dag, selection).map_or(canvas, |t| {
            canvas.centered_on(t.x, t.y, t.width, t.height, area)
        })
    }
}

fn find_scroll_target<'a>(
    layout: &'a TreeLayout,
    dag: &CompleteFunctionalDag,
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

fn render_theorem_pane(
    dag: &CompleteFunctionalDag,
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
            " Definition ",
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

    Paragraph::new(format!("⊢ {}", truncate_str(&type_text, 50)))
        .style(style)
        .render(inner, buf);

    state.click_regions.push(ClickRegion {
        area: inner,
        selection: Selection::Theorem,
    });
}
