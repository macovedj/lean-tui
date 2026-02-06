use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use super::{
    state_node::{EffectStateNode, EffectStateNodeState},
    types::EffectFlowDag,
};
use crate::{
    graph_layout::{calculate_layout, GraphViewport, LayoutConfig},
    shared::{build_navigation_regions, Selection},
};

pub type GraphPaneState = GraphViewport;

pub fn render_graph_pane(
    dag: &EffectFlowDag,
    config: LayoutConfig,
    selection: Option<Selection>,
    area: Rect,
    buf: &mut Buffer,
    state: &mut GraphPaneState,
) {
    state.clear_click_regions();

    if dag.is_empty() || dag.root_node_id.is_none() {
        Paragraph::new("No effect flow steps")
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
            let is_selected =
                matches!(selection, Some(Selection::Goal { node_id, .. }) if node_id == node.id);
            let mut node_state = EffectStateNodeState::default();
            EffectStateNode::new(node, is_current, is_selected).render(
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
