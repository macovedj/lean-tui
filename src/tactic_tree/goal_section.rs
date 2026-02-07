use std::collections::HashSet;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

use crate::{
    deduction_tree::{ProofObligation, TacticProofState},
    protocol::NodeId,
    shared::{
        diff_text::AnnotatedTextTreeExt,
        theme::{Orange, Theme},
        Region, Selection,
    },
};

/// State for the goal section widget.
#[derive(Default)]
pub struct GoalSectionState {
    goals: Vec<ProofObligation>,
    selection: Option<Selection>,
    spawned_goal_ids: HashSet<String>,
    /// Node ID for creating click region selections.
    node_id: Option<NodeId>,
    /// Name of the goal the cursor's tactic is working on.
    active_goal_name: Option<String>,
    click_regions: Vec<Region>,
    scroll_state: ScrollbarState,
    vertical_scroll: usize,
    /// Whether this pane is currently focused.
    is_focused: bool,
}

/// Lay out information for tracking click regions in the goal section.
struct GoalClickLayout {
    area: Rect,
    goal_count: usize,
}

impl GoalSectionState {
    pub fn update(
        &mut self,
        state: &TacticProofState,
        selection: Option<Selection>,
        spawned_goal_ids: HashSet<String>,
        node_id: Option<NodeId>,
        active_goal_name: Option<String>,
    ) {
        self.goals.clone_from(&state.goals);
        self.selection = selection;
        self.spawned_goal_ids = spawned_goal_ids;
        self.node_id = node_id;
        self.active_goal_name = active_goal_name;
    }

    /// Get the click regions computed during the last render.
    pub fn click_regions(&self) -> &[Region] {
        &self.click_regions
    }

    pub const fn set_focused(&mut self, focused: bool) {
        self.is_focused = focused;
    }
}

/// Widget for rendering the goal section.
pub struct GoalSection;

impl StatefulWidget for GoalSection {
    type State = GoalSectionState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        state.click_regions.clear();

        let border_color = if state.is_focused {
            Orange::BRIGHT
        } else {
            Orange::MUTED
        };
        let title = if state.is_focused {
            "▼ Goals "
        } else {
            "  Goals "
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(border_color))
            .title(title)
            .title_style(Style::new().fg(Theme::TITLE_GOAL));

        let inner = block.inner(area);
        block.render(area, buf);

        if state.goals.is_empty() {
            Paragraph::new("✓ All goals completed!")
                .style(Style::new().fg(Theme::COMPLETED_GOAL_FG).add_modifier(Modifier::BOLD))
                .render(inner, buf);
            return;
        }

        if let Some(Selection::Goal { goal_idx, .. }) = state.selection {
            state.vertical_scroll = goal_idx;
        }

        let total_goals = state.goals.len();
        let viewport_height = inner.height as usize;

        state.scroll_state = state
            .scroll_state
            .content_length(total_goals)
            .position(state.vertical_scroll);

        if let Some(node_id) = state.node_id {
            let layout = GoalClickLayout {
                area: inner,
                goal_count: state.goals.len(),
            };
            track_goal_click_regions(&mut state.click_regions, node_id, &layout);
        }

        let lines: Vec<Line<'static>> = state
            .goals
            .iter()
            .enumerate()
            .map(|(goal_idx, goal)| {
                let is_selected = matches!(
                    state.selection,
                    Some(Selection::Goal { goal_idx: gi, .. }) if gi == goal_idx
                );
                let is_active = goal
                    .username_str()
                    .is_some_and(|name| state.active_goal_name.as_deref() == Some(name));
                goal_line(goal, is_selected, is_active)
            })
            .collect();

        Paragraph::new(lines).render(inner, buf);

        if total_goals > viewport_height && inner.width > 1 {
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y, // Goals section has bottom border
                width: 1,
                height: area.height.saturating_sub(1),
            };
            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                scrollbar_area,
                buf,
                &mut state.scroll_state,
            );
        }
    }
}

fn track_goal_click_regions(
    click_regions: &mut Vec<Region>,
    node_id: NodeId,
    layout: &GoalClickLayout,
) {
    click_regions.extend(
        (0..layout.goal_count)
            .map(|goal_idx| (goal_idx, layout.area.y + goal_idx as u16))
            .take_while(|&(_, y)| y < layout.area.y + layout.area.height)
            .map(|(goal_idx, y)| {
                Region::from_rect(
                    Rect::new(layout.area.x, y, layout.area.width, 1),
                    Selection::Goal { node_id, goal_idx },
                )
            }),
    );
}

fn goal_line(goal: &ProofObligation, is_selected: bool, is_active: bool) -> Line<'static> {
    let base_color = if is_active {
        Theme::CURRENT_NODE_BORDER // White for active goal
    } else {
        Theme::GOAL_FG // Bright orange for goals
    };
    let style = if is_selected {
        Style::new().bg(Theme::SELECTION_BG).fg(base_color)
    } else {
        Style::new().fg(base_color)
    };

    let case_label = goal
        .username_str()
        .map_or(String::new(), |n| format!("{n}: "));

    let mut spans = vec![Span::styled(case_label, style), Span::styled("⊢ ", style)];
    spans.extend(goal.type_.to_spans(style));
    Line::from(spans)
}
