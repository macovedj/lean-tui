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

use super::state_node::{EffectStateNode, EffectStateNodeState};
use crate::{
    data_flow::given_pane::{BindingGivenPane, BindingGivenPaneState},
    protocol::{GenericDag, NodeId},
    shared::{
        build_navigation_regions, calculate_dag_layout, theme::Theme, truncate_str, ClickRegion,
        DagLayout, LayoutConfig, NavigationRegion, NodePosition, Selection, VirtualCanvas,
    },
};

/// State for the effect flow tableau layout.
#[derive(Default)]
pub struct EffectTableauState {
    pub given: BindingGivenPaneState,
    pub proof: EffectProofPaneState,
    pub theorem: EffectTheoremPaneState,
}

impl EffectTableauState {
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

    /// Toggle stretch mode for children layout.
    pub fn toggle_stretch(&mut self) {
        self.proof.stretch_children = !self.proof.stretch_children;
    }

    /// Check if stretch mode is enabled.
    pub const fn stretch_enabled(&self) -> bool {
        self.proof.stretch_children
    }
}

/// State for the effect proof pane.
#[derive(Default)]
pub struct EffectProofPaneState {
    scroll_x: i32,
    scroll_y: i32,
    manual_scroll: bool,
    prev_current_node: Option<NodeId>,
    layout: DagLayout,
    pub stretch_children: bool,
    pub click_regions: Vec<ClickRegion>,
    pub navigation_regions: Vec<NavigationRegion>,
}

impl EffectProofPaneState {
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
pub struct EffectTheoremPaneState {
    click_regions: Vec<ClickRegion>,
}

impl EffectTheoremPaneState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        let point = Rect::new(x, y, 1, 1);
        self.click_regions
            .iter()
            .find(|r| r.area.intersects(point))
            .map(|r| r.selection)
    }
}

/// Effect flow tableau layout widget.
pub struct EffectTableauLayout<'a> {
    dag: &'a GenericDag,
    config: LayoutConfig,
    selection: Option<Selection>,
}

impl<'a> EffectTableauLayout<'a> {
    pub const fn new(
        dag: &'a GenericDag,
        top_down: bool,
        stretch_children: bool,
        selection: Option<Selection>,
    ) -> Self {
        Self {
            dag,
            config: LayoutConfig {
                top_down,
                stretch_children,
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

impl StatefulWidget for EffectTableauLayout<'_> {
    type State = EffectTableauState;

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
    dag: &GenericDag,
    config: LayoutConfig,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut EffectProofPaneState,
) {
    state.click_regions.clear();
    state.navigation_regions.clear();

    if dag.is_empty() || dag.root_node_id.is_none() {
        Paragraph::new("No effect flow steps")
            .style(Style::new().fg(Color::DarkGray))
            .render(area, buf);
        return;
    }

    let layout_config = LayoutConfig {
        top_down: config.top_down,
        stretch_children: state.stretch_children,
    };
    state.layout = calculate_dag_layout(dag, layout_config);
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
            let mut node_state = EffectStateNodeState::default();
            EffectStateNode::new(
                node,
                dag.is_current(pos.node_id),
                selection,
                config.top_down,
            )
            .render(render_area, buf, &mut node_state);
            node_state.click_regions
        })
        .collect();

    state.click_regions = click_regions;

    render_scrollbars(buf, area, &canvas);
}

fn create_canvas(
    state: &EffectProofPaneState,
    dag: &GenericDag,
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
    layout: &'a DagLayout,
    dag: &GenericDag,
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
    dag: &GenericDag,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut EffectTheoremPaneState,
) {
    state.click_regions.clear();

    let is_selected = matches!(selection, Some(Selection::Theorem));
    let border_style = Style::new().fg(if is_selected {
        Theme::BORDER_FOCUSED
    } else {
        Theme::BORDER
    });

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(
            " Result Type ",
            Style::new().fg(Theme::TITLE_GOAL),
        ));

    let inner = block.inner(area);
    block.render(area, buf);

    state.click_regions.push(ClickRegion {
        area: inner,
        selection: Selection::Theorem,
    });

    let type_text = dag.definition_type.as_ref().map_or_else(
        || "?".to_string(),
        |t| truncate_str(&t.to_plain_text(), inner.width.saturating_sub(2) as usize),
    );

    let style = Style::new()
        .fg(Theme::GOAL_TYPE)
        .add_modifier(if is_selected {
            Modifier::UNDERLINED
        } else {
            Modifier::empty()
        });

    Paragraph::new(Span::styled(type_text, style)).render(inner, buf);
}
