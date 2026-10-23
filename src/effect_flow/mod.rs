pub mod graph;

use std::iter::once;

pub use graph::{EdgeKind, EffectFlowConversionError, EffectFlowDag, EffectKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::{
    graph_layout::{calculate_layout, draw_graph_scene, DagLayout, GraphScene, GraphSelection},
    message::{ClickTarget, ModeAction},
    protocol::TimingDisplay,
    selection::Selection,
    theme::Theme,
    widgets::{
        legend::{LegendItem, LegendSection},
        node_content::NodeContentWidget,
    },
};

#[derive(Debug, Clone, Copy, Default)]
pub struct EffectFlowView {
    pub selection: GraphSelection,
    pub expanded: bool,
}

impl EffectFlowView {
    pub const fn toggle(&mut self, action: ModeAction) {
        if matches!(action, ModeAction::ToggleExpanded) {
            self.expanded = !self.expanded;
        }
    }
}

pub fn effect_flow_selectable_items(dag: &EffectFlowDag) -> Vec<Selection> {
    dag.dfs_iter()
        .flat_map(|node| {
            let node_id = node.id;
            let hyp_selections = (0..node.visible_bindings().len())
                .map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });
            hyp_selections.chain(once(Selection::Goal {
                node_id,
                goal_idx: 0,
            }))
        })
        .collect()
}

fn effect_flow_layout(dag: &EffectFlowDag, expanded: bool, timing: TimingDisplay) -> DagLayout {
    calculate_layout(&dag.to_layout_graph_filtered(expanded, timing), true)
}

pub fn current_effect_flow_selection(
    view: &EffectFlowView,
    dag: &EffectFlowDag,
) -> Option<Selection> {
    view.selection
        .current(dag, &effect_flow_selectable_items(dag))
}

pub fn apply_effect_flow_action(
    view: &mut EffectFlowView,
    action: ModeAction,
    dag: &EffectFlowDag,
    timing: TimingDisplay,
) {
    if let ModeAction::Move { direction } = action {
        let layout = effect_flow_layout(dag, view.expanded, timing);
        view.selection
            .step(dag, &layout, &effect_flow_selectable_items(dag), direction);
    } else {
        view.toggle(action);
    }
}

fn render_monad_stack(dag: &EffectFlowDag, area: Rect, buf: &mut Buffer) -> Rect {
    let Some(monad_stack) = dag.monad_stack().filter(|stack| !stack.is_empty()) else {
        return area;
    };

    let spans = vec![
        Span::styled("Monad: ", Style::new().fg(Color::DarkGray)),
        Span::styled(
            monad_stack.join(" → "),
            Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
    ];
    Paragraph::new(Line::from(spans)).render(area, buf);

    if area.height > 1 {
        Rect::new(area.x, area.y + 1, area.width, area.height - 1)
    } else {
        area
    }
}

pub fn draw_effect_flow(
    view: &EffectFlowView,
    dag: &EffectFlowDag,
    timing: TimingDisplay,
    area: Rect,
    buf: &mut Buffer,
) -> Vec<ClickTarget> {
    let graph_area = render_monad_stack(dag, area, buf);

    if dag.is_empty() || dag.root_node_id.is_none() {
        Paragraph::new("No effect flow steps")
            .style(Style::new().fg(Color::DarkGray))
            .render(graph_area, buf);
        return Vec::new();
    }

    let selection = current_effect_flow_selection(view, dag);
    let scene = GraphScene {
        graph: &dag.to_graph_filtered(view.expanded),
        layout: &effect_flow_layout(dag, view.expanded, timing),
        selection,
        top_down: true,
    };
    draw_graph_scene(&scene, graph_area, buf, |node, is_current| {
        NodeContentWidget {
            content: node.build_content(selection, is_current, timing),
            style: node.build_node_style(is_current),
        }
    })
}

pub fn effect_flow_legend_sections() -> Vec<LegendSection> {
    vec![
        LegendSection::new(
            "Nodes",
            Color::White,
            vec![
                LegendItem::new("■", Theme::CURRENT_NODE_BORDER, "current"),
                LegendItem::new(EffectKind::EffectBind.icon(), Theme::EFFECT_BIND, "bind"),
                LegendItem::new(
                    EffectKind::StateRead.icon(),
                    Theme::EFFECT_STATE_READ,
                    "read",
                ),
                LegendItem::new(
                    EffectKind::StateWrite.icon(),
                    Theme::EFFECT_STATE_WRITE,
                    "write",
                ),
                LegendItem::new(
                    EffectKind::StateModify.icon(),
                    Theme::EFFECT_STATE_MODIFY,
                    "modify",
                ),
                LegendItem::new(
                    EffectKind::EffectReturn.icon(),
                    Theme::EFFECT_RETURN,
                    "return",
                ),
                LegendItem::new(EffectKind::EffectThrow.icon(), Theme::EFFECT_THROW, "throw"),
                LegendItem::new(EffectKind::Pure.icon(), Theme::EFFECT_PURE, "pure"),
            ],
        ),
        LegendSection::new(
            "Edges",
            Color::White,
            vec![
                LegendItem::new("─", EdgeKind::DataFlow.color(), "data"),
                LegendItem::new("─", EdgeKind::ControlFlow.color(), "control"),
                LegendItem::new("─", EdgeKind::EffectFlow.color(), "effect"),
                LegendItem::new("─", EdgeKind::ErrorPropagation.color(), "error"),
                LegendItem::new("─", EdgeKind::EarlyReturn.color(), "early return"),
                LegendItem::new("─", EdgeKind::Continuation.color(), "continuation"),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::scenario_replay::{key, render_scenario, ReplayedMessages};

    #[test]
    fn monadic_expanded_with_timing() {
        assert_snapshot!(render_scenario(
            "monadic",
            &ReplayedMessages::All,
            &[key('e'), key('m')]
        ));
    }
}
