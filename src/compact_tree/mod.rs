pub mod proof_steps_sidebar;

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Color,
    widgets::Widget,
};

use crate::{
    compact_tree::proof_steps_sidebar::{
        max_step_line_width, required_sidebar_width, ProofStepsSidebar,
    },
    deduction_tree::{
        active_goal_name, current_step, CompleteProofDag, StepSelection, TacticProofState,
    },
    message::{ClickTarget, ModeAction},
    navigation::Direction,
    selection::{next_list_index, previous_list_index},
    theme::Theme,
    widgets::{
        errors_section::ErrorsSection,
        legend::{LegendItem, LegendSection},
        render_helpers::render_no_goals,
        AssumptionPane, GoalsPane,
    },
};

#[derive(Debug, Clone, Copy, Default)]
pub struct CompactTreeView {
    pub selection: StepSelection,
    pub horizontal_scroll: usize,
}

pub fn apply_compact_tree_action(
    view: &mut CompactTreeView,
    action: ModeAction,
    dag: &CompleteProofDag,
) {
    let ModeAction::Move { direction } = action else {
        return;
    };
    match direction {
        Direction::Down => view.selection.step(dag, next_list_index),
        Direction::Up => view.selection.step(dag, previous_list_index),
        Direction::Left => view.horizontal_scroll = view.horizontal_scroll.saturating_sub(1),
        Direction::Right => {
            view.horizontal_scroll = (view.horizontal_scroll + 1).min(max_step_line_width(dag));
        }
    }
}

pub fn draw_compact_tree(
    view: &CompactTreeView,
    dag: &CompleteProofDag,
    content: Rect,
    buf: &mut Buffer,
) -> Vec<ClickTarget> {
    let step = current_step(dag);
    let default_state = TacticProofState::default();
    let state = step.map_or(&default_state, |node| &node.proof_state);

    if state.goals.is_empty() && dag.is_empty() {
        render_no_goals(content, buf);
        return Vec::new();
    }

    let step_errors = step.map_or(&[][..], |node| &node.diagnostics);
    let error_line_count: usize = step_errors
        .iter()
        .map(|error| error.message.lines().count())
        .sum();
    let error_height = if error_line_count > 0 {
        ((error_line_count as u16) + 2).min((content.height / 3).max(3))
    } else {
        0
    };
    let [main_area, errors_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(error_height)]).areas(content);

    let sidebar_width = required_sidebar_width(dag)
        .max(20)
        .min((main_area.width * 60) / 100);
    let [sidebar_area, right_area] =
        Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Fill(1)])
            .areas(main_area);
    ProofStepsSidebar {
        dag,
        horizontal_scroll: view.horizontal_scroll,
    }
    .render(sidebar_area, buf);

    let [hyp_area, goal_area] =
        Layout::vertical([Constraint::Percentage(40), Constraint::Percentage(60)])
            .areas(right_area);

    let current_node_id = dag.current_node_id;
    let is_current = current_node_id.is_some();
    let selection = view.selection.current(dag);

    let assumption_targets =
        AssumptionPane::new(state, selection, true, is_current, current_node_id)
            .draw(hyp_area, buf);
    let goal_targets = GoalsPane::new(
        state,
        selection,
        true,
        is_current,
        current_node_id,
        step.and_then(active_goal_name),
    )
    .draw(goal_area, buf);
    let error_targets = if error_line_count > 0 {
        ErrorsSection {
            errors: step_errors,
        }
        .draw(errors_area, buf)
    } else {
        Vec::new()
    };

    assumption_targets
        .into_iter()
        .chain(goal_targets)
        .chain(error_targets)
        .collect()
}

pub fn compact_tree_legend_sections() -> Vec<LegendSection> {
    vec![LegendSection::new(
        "Compact Tree",
        Color::Yellow,
        vec![
            LegendItem::new("▶", Theme::CURRENT_NODE_BORDER, "current step"),
            LegendItem::new("⊢", Theme::GOAL, "open goal"),
        ],
    )]
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::scenario_replay::{key, render_scenario, ReplayedMessages};

    #[test]
    fn prime_steps_sidebar() {
        assert_snapshot!(render_scenario(
            "prime",
            &ReplayedMessages::All,
            &[key('[')]
        ));
    }
}
