mod diff_pane;

use diff_pane::{DiffPane, DiffPaneKind};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Color,
};

use crate::{
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
    },
};

#[derive(Debug, Clone, Copy)]
pub struct BeforeAfterView {
    pub selection: StepSelection,
    pub show_previous: bool,
    pub show_next: bool,
}

impl Default for BeforeAfterView {
    fn default() -> Self {
        Self {
            selection: StepSelection::default(),
            show_previous: true,
            show_next: false,
        }
    }
}

pub fn apply_before_after_action(
    view: &mut BeforeAfterView,
    action: ModeAction,
    dag: &CompleteProofDag,
) {
    match action {
        ModeAction::Move {
            direction: Direction::Down,
        } => view.selection.step(dag, next_list_index),
        ModeAction::Move {
            direction: Direction::Up,
        } => view.selection.step(dag, previous_list_index),
        ModeAction::TogglePreviousPane => view.show_previous = !view.show_previous,
        ModeAction::ToggleNextPane => view.show_next = !view.show_next,
        _ => {}
    }
}

fn column_constraints(has_previous: bool, has_next: bool) -> Vec<Constraint> {
    match (has_previous, has_next) {
        (true, true) => vec![
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ],
        (true, false) => vec![Constraint::Percentage(35), Constraint::Percentage(65)],
        (false, true) => vec![Constraint::Percentage(65), Constraint::Percentage(35)],
        (false, false) => vec![Constraint::Percentage(100)],
    }
}

pub fn draw_before_after(
    view: &BeforeAfterView,
    dag: &CompleteProofDag,
    content_area: Rect,
    buf: &mut Buffer,
) -> Vec<ClickTarget> {
    let step = current_step(dag);
    let step_errors = step.map_or(&[][..], |node| &node.diagnostics);
    let error_line_count: usize = step_errors
        .iter()
        .map(|error| error.message.lines().count())
        .sum();

    let [main_area, errors_area] = Layout::vertical([
        Constraint::Fill(1),
        Constraint::Length(if error_line_count > 0 {
            ((error_line_count as u16) + 2).min(content_area.height / 2)
        } else {
            0
        }),
    ])
    .areas(content_area);

    let default_state = TacticProofState::default();
    let current_state = step.map_or(&default_state, |node| &node.proof_state);
    let previous_state = step
        .filter(|_| view.show_previous)
        .and_then(|node| node.parent)
        .and_then(|parent_id| dag.get(parent_id))
        .map(|parent| &parent.proof_state);
    let next_state = step
        .filter(|_| view.show_next)
        .and_then(|node| node.children.first())
        .and_then(|&child_id| dag.get(child_id))
        .map(|child| &child.proof_state);

    let goal_name = step.and_then(active_goal_name);
    let selection = view.selection.current(dag);
    let current_node_id = dag.current_node_id;

    let columns = Layout::horizontal(column_constraints(
        previous_state.is_some(),
        next_state.is_some(),
    ))
    .split(main_area);
    let panes = [
        previous_state
            .map(|state| DiffPane::new(DiffPaneKind::Before, state, None, None, goal_name)),
        Some(DiffPane::new(
            DiffPaneKind::Current,
            current_state,
            selection,
            current_node_id,
            goal_name,
        )),
        next_state.map(|state| DiffPane::new(DiffPaneKind::After, state, None, None, goal_name)),
    ];
    let pane_targets: Vec<ClickTarget> = panes
        .into_iter()
        .flatten()
        .zip(columns.iter())
        .flat_map(|(pane, &column)| pane.draw(column, buf))
        .collect();

    let error_targets = if error_line_count > 0 {
        ErrorsSection {
            errors: step_errors,
        }
        .draw(errors_area, buf)
    } else {
        Vec::new()
    };

    pane_targets.into_iter().chain(error_targets).collect()
}

pub fn before_after_legend_sections() -> Vec<LegendSection> {
    vec![
        LegendSection::new(
            "Diff markers",
            Color::White,
            vec![
                LegendItem::new("x", Theme::UNCHANGED, "unchanged"),
                LegendItem::new("+", Theme::ADDED, "added"),
                LegendItem::new("~", Theme::CHANGED, "changed"),
                LegendItem::new("-", Theme::REMOVED, "removed"),
            ],
        ),
        LegendSection::new(
            "Elements",
            Color::White,
            vec![
                LegendItem::new("h", Theme::HYP_NAME, "hypothesis"),
                LegendItem::new("⊢", Theme::GOAL, "goal"),
            ],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::scenario_replay::{click, key, render_scenario, ReplayedMessages};

    #[test]
    fn swap_clicked_hypotheses_and_errors() {
        assert_snapshot!(render_scenario(
            "swap",
            &ReplayedMessages::All,
            &[
                key('['),
                key('['),
                click(10, 5),
                click(60, 8),
                click(5, 40),
                click(70, 38)
            ]
        ));
    }

    #[test]
    fn swap_with_next_step_shown() {
        assert_snapshot!(render_scenario(
            "swap",
            &ReplayedMessages::All,
            &[key('['), key('['), key('n'), key('j')]
        ));
    }
}
