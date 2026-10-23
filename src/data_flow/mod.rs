pub mod graph;

use std::{collections::HashSet, iter::once};

pub use graph::{DataFlowConversionError, DataFlowDag};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

use crate::{
    graph_layout::{calculate_layout, draw_graph_scene, DagLayout, GraphScene, TreeGraphView},
    message::{ClickTarget, ModeAction},
    protocol::{NodeId, TimingDisplay},
    selection::Selection,
    theme::Theme,
    widgets::{
        legend::{LegendItem, LegendSection},
        node_content::NodeContentWidget,
    },
};

pub fn data_flow_selectable_items(dag: &DataFlowDag) -> Vec<Selection> {
    dag.dfs_iter()
        .flat_map(|node| {
            let node_id = node.id;
            let binding_count = node
                .bindings()
                .iter()
                .filter(|b| !b.is_implicit && !b.is_instance)
                .count();
            let hyp_selections =
                (0..binding_count).map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });
            hyp_selections.chain(once(Selection::Goal {
                node_id,
                goal_idx: 0,
            }))
        })
        .collect()
}

fn data_flow_layout(dag: &DataFlowDag, top_down: bool, timing: TimingDisplay) -> DagLayout {
    calculate_layout(&dag.to_layout_graph(timing), top_down)
}

pub fn data_flow_error_message(
    dag: &DataFlowDag,
    node_id: NodeId,
    error_idx: usize,
) -> Option<String> {
    dag.get(node_id)
        .and_then(|node| node.diagnostics.get(error_idx))
        .map(|diagnostic| diagnostic.message.clone())
}

pub fn current_data_flow_selection(view: &TreeGraphView, dag: &DataFlowDag) -> Option<Selection> {
    view.selection
        .current(dag, &data_flow_selectable_items(dag))
}

pub fn apply_data_flow_action(
    view: &mut TreeGraphView,
    action: ModeAction,
    dag: &DataFlowDag,
    timing: TimingDisplay,
) {
    if let ModeAction::Move { direction } = action {
        let layout = data_flow_layout(dag, view.top_down, timing);
        view.selection
            .step(dag, &layout, &data_flow_selectable_items(dag), direction);
    } else {
        view.toggle(action);
    }
}

pub fn draw_data_flow(
    view: &TreeGraphView,
    dag: &DataFlowDag,
    timing: TimingDisplay,
    area: Rect,
    buf: &mut Buffer,
) -> Vec<ClickTarget> {
    if dag.is_empty() || dag.root_node_id.is_none() {
        Paragraph::new("No expression steps")
            .style(Style::new().fg(Color::DarkGray))
            .render(area, buf);
        return Vec::new();
    }

    let selection = current_data_flow_selection(view, dag);
    let scene = GraphScene {
        graph: &dag.to_graph(),
        layout: &data_flow_layout(dag, view.top_down, timing),
        selection,
        top_down: view.top_down,
    };
    draw_graph_scene(&scene, area, buf, |node, is_current| NodeContentWidget {
        content: node.build_content(selection, is_current, view.top_down, timing),
        style: node.build_node_style(is_current),
    })
}

pub fn node_kind_legend_items(dag: &DataFlowDag) -> Vec<LegendItem> {
    let mut seen = HashSet::new();
    dag.nodes
        .iter()
        .filter_map(|node| {
            let kind = node.data_flow_kind();
            let description = kind.description().to_owned();
            seen.insert(description.clone())
                .then(|| LegendItem::new(kind.icon(), kind.color(), description))
        })
        .collect()
}

pub fn data_flow_legend_sections(node_kinds: Vec<LegendItem>) -> Vec<LegendSection> {
    vec![
        LegendSection::new("Node Kinds", Color::White, node_kinds),
        LegendSection::new(
            "Node States",
            Color::White,
            vec![
                LegendItem::new("■", Theme::CURRENT_NODE_BORDER, "current"),
                LegendItem::new("■", Theme::COMPLETED_GOAL_FG, "complete"),
                LegendItem::new("■", Theme::INCOMPLETE_NODE_BORDER, "incomplete"),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::scenario_replay::{key, render_scenario, ReplayedMessages};

    #[test]
    fn sorting_navigated_with_hjkl() {
        assert_snapshot!(render_scenario(
            "sorting",
            &ReplayedMessages::All,
            &[key('j'), key('l'), key('j'), key('h')]
        ));
    }
}
