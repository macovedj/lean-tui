//! Proof steps sidebar widget for the tactic tree view.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget,
        Widget,
    },
};

use crate::{
    deduction_tree::types::{CompleteProofDag, ProofDagTacticNode},
    protocol::NodeId,
    shared::{theme::Theme, InteractiveStatefulWidget},
};

const BRANCH_MID: &str = "├─ ";
const BRANCH_END: &str = "╰─ ";
const VERT_LINE: &str = "│  ";
const EMPTY: &str = "   ";

/// State for the proof steps sidebar widget.
#[derive(Default)]
pub struct ProofStepsSidebarState {
    proof_dag: Option<CompleteProofDag>,
    scroll_state: ScrollbarState,
    vertical_scroll: usize,
    horizontal_scroll: usize,
    /// Whether the user has taken manual control of scrolling.
    manual_scroll: bool,
    /// Previous current node ID, used to detect changes.
    prev_current_node: Option<NodeId>,
    /// Max content width (cached from last render).
    max_content_width: usize,
    /// Viewport width (cached from last render).
    viewport_width: usize,
}

impl ProofStepsSidebarState {
    fn build_lines(&self) -> Vec<Line<'static>> {
        let Some(dag) = &self.proof_dag else {
            return vec![];
        };

        // Track which depth levels are still "active" (have more siblings to come)
        let mut active_levels: Vec<bool> = Vec::new();
        let mut lines = Vec::new();

        for node in dag.dfs_iter() {
            let is_current = dag.is_current(node.id);

            // Determine if this node is the last child of its parent
            let is_last_child = node
                .parent
                .and_then(|pid| dag.get(pid))
                .is_none_or(|parent| parent.children.last() == Some(&node.id));

            // Adjust active_levels to match current depth
            let depth = node.depth as usize;
            update_active_levels(&mut active_levels, depth, is_last_child);

            // Build the tree prefix
            let prefix = build_tree_prefix(&active_levels, depth, is_last_child);
            lines.push(step_line(node, &prefix, is_current));

            // For continuation lines, use the same prefix structure but with vertical lines
            let cont_prefix = build_continuation_prefix(&active_levels, depth, is_last_child);

            if let Some(deps) = dependency_line(node, &cont_prefix) {
                lines.push(deps);
            }
            if let Some(thms) = theorem_line(node, &cont_prefix) {
                lines.push(thms);
            }
        }

        lines
    }

    /// Calculate scroll position to center the current step in the viewport.
    fn calculate_centered_scroll(&self, viewport_height: usize) -> usize {
        let Some(dag) = &self.proof_dag else {
            return 0;
        };

        let current_line: usize = dag
            .dfs_iter()
            .take_while(|node| !dag.is_current(node.id))
            .map(|node| {
                1 + usize::from(!node.hypothesis_dependencies.is_empty())
                    + usize::from(!node.referenced_theorems.is_empty())
            })
            .sum();
        let half_viewport = viewport_height / 2;
        let total = self.total_lines();
        let max_scroll = total.saturating_sub(viewport_height);

        current_line.saturating_sub(half_viewport).min(max_scroll)
    }

    /// Total number of lines in the proof steps view.
    fn total_lines(&self) -> usize {
        let Some(dag) = &self.proof_dag else {
            return 0;
        };

        dag.dfs_iter()
            .map(|node| {
                1 + usize::from(!node.hypothesis_dependencies.is_empty())
                    + usize::from(!node.referenced_theorems.is_empty())
            })
            .sum()
    }

    pub const fn scroll_left(&mut self) {
        self.manual_scroll = true;
        self.horizontal_scroll = self.horizontal_scroll.saturating_sub(1);
    }

    pub const fn scroll_right(&mut self) {
        self.manual_scroll = true;
        let max_horiz = self.max_content_width.saturating_sub(self.viewport_width);
        if self.horizontal_scroll < max_horiz {
            self.horizontal_scroll += 1;
        }
    }

    /// Calculate the required width for the sidebar based on content.
    /// Returns the width needed to display all lines without horizontal scrolling,
    /// plus borders and scrollbar.
    pub fn required_width(&self) -> u16 {
        let lines = self.build_lines();
        let max_content = lines.iter().map(Line::width).max().unwrap_or(0);
        // Add 4 for borders (2) + scrollbar (1) + padding (1)
        (max_content as u16).saturating_add(4)
    }
}

fn update_active_levels(active_levels: &mut Vec<bool>, depth: usize, is_last_child: bool) {
    active_levels.truncate(depth);
    if depth > 0 {
        active_levels.resize(depth, true);
        if let Some(last) = active_levels.last_mut() {
            *last = !is_last_child;
        }
    }
}

fn build_tree_prefix(active_levels: &[bool], depth: usize, is_last_child: bool) -> String {
    if depth == 0 {
        return String::new();
    }

    let mut prefix = String::new();

    for &active in active_levels.iter().take(depth.saturating_sub(1)) {
        prefix.push_str(if active { VERT_LINE } else { EMPTY });
    }

    prefix.push_str(if is_last_child {
        BRANCH_END
    } else {
        BRANCH_MID
    });

    prefix
}

/// Build the prefix for continuation lines (uses, thms).
fn build_continuation_prefix(active_levels: &[bool], depth: usize, is_last_child: bool) -> String {
    if depth == 0 {
        return String::from("   "); // Indent for root node's continuation
    }

    let mut prefix = String::new();

    for &active in active_levels.iter().take(depth.saturating_sub(1)) {
        prefix.push_str(if active { VERT_LINE } else { EMPTY });
    }

    prefix.push_str(if is_last_child { EMPTY } else { VERT_LINE });

    prefix
}

/// Widget displaying proof steps in a sidebar.
pub struct ProofStepsSidebar;

impl InteractiveStatefulWidget for ProofStepsSidebar {
    type Input = Option<CompleteProofDag>;
    type Event = ();

    fn update_state(state: &mut Self::State, input: Self::Input) {
        let new_current = input.as_ref().and_then(|dag| dag.current_node_id);

        if new_current != state.prev_current_node {
            state.manual_scroll = false;
            state.prev_current_node = new_current;
        }

        state.proof_dag = input;
    }
}

impl StatefulWidget for ProofStepsSidebar {
    type State = ProofStepsSidebarState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Theme::INACTIVE_NODE_BORDER))
            .title(" Steps ")
            .title_style(Style::new().fg(Theme::TITLE_TACTIC));

        let inner = block.inner(area);
        block.render(area, buf);

        let lines = state.build_lines();
        let total_lines = lines.len();
        let viewport_height = inner.height as usize;
        let viewport_width = inner.width as usize;

        state.viewport_width = viewport_width;
        state.max_content_width = lines.iter().map(Line::width).max().unwrap_or(0);

        if !state.manual_scroll {
            state.vertical_scroll = state.calculate_centered_scroll(viewport_height);
        }

        let max_vert_scroll = total_lines.saturating_sub(viewport_height);
        let max_horiz_scroll = state.max_content_width.saturating_sub(viewport_width);
        state.vertical_scroll = state.vertical_scroll.min(max_vert_scroll);
        state.horizontal_scroll = state.horizontal_scroll.min(max_horiz_scroll);

        state.scroll_state = state
            .scroll_state
            .content_length(total_lines)
            .position(state.vertical_scroll);

        Paragraph::new(lines)
            .scroll((state.vertical_scroll as u16, state.horizontal_scroll as u16))
            .render(inner, buf);

        if total_lines > viewport_height && inner.width > 1 {
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y + 1, // Skip top border
                width: 1,
                height: area.height.saturating_sub(2), // Skip top and bottom borders
            };
            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                scrollbar_area,
                buf,
                &mut state.scroll_state,
            );
        }
    }
}

fn step_line(node: &ProofDagTacticNode, prefix: &str, is_current: bool) -> Line<'static> {
    let style = if is_current {
        Style::new()
            .fg(Theme::CURRENT_NODE_BORDER)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::new().fg(Color::Gray).add_modifier(Modifier::DIM)
    };

    let marker = if is_current { "▶ " } else { "  " };

    Line::from(vec![
        Span::styled(marker, Style::new().fg(Theme::CURRENT_NODE_BORDER)),
        Span::styled(prefix.to_string(), Theme::INACTIVE_NODE_BORDER),
        Span::styled(node.tactic.text.clone(), style),
    ])
}

fn dependency_line(node: &ProofDagTacticNode, prefix: &str) -> Option<Line<'static>> {
    if node.hypothesis_dependencies.is_empty() {
        return None;
    }

    Some(Line::from(vec![
        Span::raw("  "), // Match marker width
        Span::styled(prefix.to_string(), Theme::INACTIVE_NODE_BORDER),
        Span::styled(
            format!("uses: {}", node.hypothesis_dependencies.join(", ")),
            Theme::DEPENDENCY,
        ),
    ]))
}

fn theorem_line(node: &ProofDagTacticNode, prefix: &str) -> Option<Line<'static>> {
    if node.referenced_theorems.is_empty() {
        return None;
    }

    Some(Line::from(vec![
        Span::raw("  "), // Match marker width
        Span::styled(prefix.to_string(), Theme::INACTIVE_NODE_BORDER),
        Span::styled(
            format!("thms: {}", node.referenced_theorems.join(", ")),
            Style::new().fg(Theme::HYP_NAME).add_modifier(Modifier::DIM),
        ),
    ]))
}
