//! Effect flow tableau layout - coordinates the panes.
//!
//! Layout structure:
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Given: parameters                                           │
//! ├─────────────────────────────────────────────────────────────┤
//! │                                                             │
//! │                     Effect Flow Graph                       │
//! │                                                             │
//! │  ⊛ deposit ──→ ⊛ withdraw ──→ ↵ return                     │
//! │       │                                                     │
//! │       ▼                                                     │
//! │  ◁ get state                                               │
//! │                                                             │
//! ├─────────────────────────────────────────────────────────────┤
//! │ Result Type: ⊢ BankM Int                                    │
//! └─────────────────────────────────────────────────────────────┘
//! ```

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};

use super::{
    graph_pane::{render_graph_pane, GraphPaneState},
    types::EffectFlowDag,
};
use crate::{
    data_flow::given_pane::{BindingGivenPane, BindingGivenPaneState},
    graph::LayoutConfig,
    protocol::NodeId,
    shared::{theme::Theme, ClickRegion, NavigationRegion, Selection},
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

    pub fn navigation_regions(&self) -> Vec<NavigationRegion> {
        self.graph.navigation_regions().to_vec()
    }
}

/// Computed layout areas.
struct LayoutAreas {
    given: Rect,
    graph: Rect,
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

    /// Compute areas for the layout.
    fn compute_layout(&self, area: Rect) -> LayoutAreas {
        let has_result = self.dag.definition_type.is_some();

        let [given_area, graph_area, result_area] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(if has_result { 3 } else { 0 }),
        ])
        .areas(area);

        LayoutAreas {
            given: given_area,
            graph: graph_area,
            result: result_area,
        }
    }
}

impl StatefulWidget for EffectTableauLayout<'_> {
    type State = EffectTableauState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let areas = self.compute_layout(area);

        if areas.given.height > 0 {
            BindingGivenPane::new(&self.dag.initial_context, self.selection).render(
                areas.given,
                buf,
                &mut state.given,
            );
        }

        render_graph_pane(
            self.dag,
            LayoutConfig {
                top_down: true,
                ..Default::default()
            },
            self.selection,
            areas.graph,
            buf,
            &mut state.graph,
        );

        if areas.result.height > 0 {
            let is_selected = matches!(self.selection, Some(Selection::Theorem));
            render_result_pane(self.dag, is_selected, areas.result, buf, &mut state.result);
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
