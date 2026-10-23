pub mod graph;

pub use graph::{
    CompleteProofDag, GoalState, ProofDagConversionError, ProofDagTacticNode, TacticProofState,
    EMPTY_PROOF_DAG,
};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::{Paragraph, Widget},
};

use crate::{
    graph_layout::{calculate_layout, draw_graph_scene, DagLayout, GraphScene, TreeGraphView},
    message::{ClickTarget, ModeAction},
    protocol::{DiagnosticSeverity, TimingDisplay},
    selection::{AnchoredSelection, Selection},
    theme::Theme,
    widgets::{
        legend::{LegendItem, LegendSection},
        node_content::NodeContentWidget,
    },
};

pub fn tableau_selectable_items(dag: &CompleteProofDag) -> Vec<Selection> {
    dag.dfs_iter_including_orphans()
        .flat_map(|node| {
            let node_id = node.id;
            let hyp_selections = (0..node.proof_state.hypotheses.len())
                .map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });
            let goal_selections = (0..node.proof_state.goals.len())
                .map(move |goal_idx| Selection::Goal { node_id, goal_idx });
            let error_count = node
                .diagnostics
                .iter()
                .filter(|d| d.severity == DiagnosticSeverity::Error)
                .count();
            let error_selections =
                (0..error_count).map(move |error_idx| Selection::Error { node_id, error_idx });
            hyp_selections
                .chain(goal_selections)
                .chain(error_selections)
        })
        .collect()
}

pub fn current_step(dag: &CompleteProofDag) -> Option<&ProofDagTacticNode> {
    dag.current_node_id.and_then(|id| dag.get(id))
}

fn step_selectable_items(step: &ProofDagTacticNode) -> Vec<Selection> {
    let node_id = step.id;
    let hyp_items = (0..step.proof_state.hypotheses.len())
        .map(move |hyp_idx| Selection::Hyp { node_id, hyp_idx });
    let goal_items = (0..step.proof_state.goals.len())
        .map(move |goal_idx| Selection::Goal { node_id, goal_idx });
    hyp_items.chain(goal_items).collect()
}

pub fn current_step_selectable_items(dag: &CompleteProofDag) -> Vec<Selection> {
    current_step(dag).map_or_else(Vec::new, step_selectable_items)
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StepShape {
    pub hypothesis_count: usize,
    pub goal_count: usize,
}

fn step_shape(dag: &CompleteProofDag) -> StepShape {
    current_step(dag).map_or_else(StepShape::default, |step| StepShape {
        hypothesis_count: step.proof_state.hypotheses.len(),
        goal_count: step.proof_state.goals.len(),
    })
}

pub type StepSelection = AnchoredSelection<usize, StepShape>;

impl StepSelection {
    fn index(&self, dag: &CompleteProofDag) -> Option<usize> {
        let shape = step_shape(dag);
        let item_count = shape.hypothesis_count + shape.goal_count;
        self.value_at(&shape)
            .or_else(|| (item_count > 0).then_some(0))
    }

    pub fn current(&self, dag: &CompleteProofDag) -> Option<Selection> {
        self.index(dag)
            .and_then(|index| current_step_selectable_items(dag).get(index).copied())
    }

    pub fn step(&mut self, dag: &CompleteProofDag, advance: fn(usize, usize) -> usize) {
        if let Some(index) = self.index(dag) {
            let item_count = current_step_selectable_items(dag).len();
            self.pin(advance(index, item_count), step_shape(dag));
        }
    }

    pub fn click(&mut self, clicked: Selection, dag: &CompleteProofDag) {
        if let Some(index) = current_step_selectable_items(dag)
            .iter()
            .position(|item| *item == clicked)
        {
            self.pin(index, step_shape(dag));
        }
    }
}

pub fn active_goal_name(step: &ProofDagTacticNode) -> Option<&str> {
    step.proof_state
        .goals
        .first()
        .and_then(GoalState::username_str)
}

fn tableau_layout(dag: &CompleteProofDag, top_down: bool, timing: TimingDisplay) -> DagLayout {
    calculate_layout(&dag.to_layout_graph(timing), top_down)
}

pub fn current_tableau_selection(
    view: &TreeGraphView,
    dag: &CompleteProofDag,
) -> Option<Selection> {
    view.selection.current(dag, &tableau_selectable_items(dag))
}

pub fn apply_tableau_action(
    view: &mut TreeGraphView,
    action: ModeAction,
    dag: &CompleteProofDag,
    timing: TimingDisplay,
) {
    if let ModeAction::Move { direction } = action {
        let layout = tableau_layout(dag, view.top_down, timing);
        view.selection
            .step(dag, &layout, &tableau_selectable_items(dag), direction);
    } else {
        view.toggle(action);
    }
}

pub fn draw_semantic_tableau(
    view: &TreeGraphView,
    dag: &CompleteProofDag,
    timing: TimingDisplay,
    area: Rect,
    buf: &mut Buffer,
) -> Vec<ClickTarget> {
    if dag.is_empty() || dag.root_node_id.is_none() {
        Paragraph::new("No proof steps")
            .style(Style::new().fg(Color::DarkGray))
            .render(area, buf);
        return Vec::new();
    }

    let selection = current_tableau_selection(view, dag);
    let scene = GraphScene {
        graph: &dag.to_graph(),
        layout: &tableau_layout(dag, view.top_down, timing),
        selection,
        top_down: view.top_down,
    };
    draw_graph_scene(&scene, area, buf, |node, is_current| NodeContentWidget {
        content: node.build_content(selection, is_current, timing),
        style: node.build_node_style(is_current),
    })
}

pub fn tableau_legend_sections() -> Vec<LegendSection> {
    vec![
        LegendSection::new(
            "Hypotheses and goals",
            Color::White,
            vec![
                LegendItem::new("h", Theme::HYP_NAME, "hypothesis"),
                LegendItem::new("P", Theme::GOAL, "goal and theorem statement"),
            ],
        ),
        LegendSection::new(
            "Proof steps",
            Color::White,
            vec![
                LegendItem::new("-", Theme::CURRENT_NODE_BORDER, "cursor"),
                LegendItem::new("-", Theme::INACTIVE_NODE_BORDER, "inactive"),
                LegendItem::new("-", Theme::COMPLETED_GOAL_FG, "completed"),
                LegendItem::new("-", Theme::INCOMPLETE_NODE_BORDER, "incomplete"),
                LegendItem::new("-", Theme::EDGE_ERROR_PROPAGATION, "type error"),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::{
        scenario_replay::{
            click, connected_model, fixture_messages, key, press_key, render_scenario,
            send_from_worker, ReplayedMessages,
        },
        selection::Selection,
    };

    #[test]
    fn swap_navigated_with_hjkl() {
        assert_snapshot!(render_scenario(
            "swap",
            &ReplayedMessages::All,
            &[key('j'), key('j'), key('l'), key('k')]
        ));
    }

    #[test]
    fn swap_clicked_nodes() {
        assert_snapshot!(render_scenario(
            "swap",
            &ReplayedMessages::All,
            &[click(70, 10), click(70, 20), click(50, 30)]
        ));
    }

    #[test]
    fn moved_selection_yields_to_active_goal_when_cursor_moves_to_another_step() {
        let mut model = connected_model();
        let mut messages = fixture_messages("prime", &ReplayedMessages::All).into_iter();
        messages
            .by_ref()
            .take(3)
            .for_each(|message| send_from_worker(&mut model, message));
        let initial_goal = model.display_mode.current_selection(model.dag());
        press_key(&mut model, 'j');
        assert_ne!(
            model.display_mode.current_selection(model.dag()),
            initial_goal
        );

        messages.for_each(|message| send_from_worker(&mut model, message));

        let current_node_id = model
            .dag()
            .proof()
            .and_then(|dag| dag.current_node_id)
            .expect("current node");
        assert_eq!(
            model.display_mode.current_selection(model.dag()),
            Some(Selection::Goal {
                node_id: current_node_id,
                goal_idx: 0
            })
        );
    }
}
