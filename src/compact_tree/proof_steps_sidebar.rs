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
    deduction_tree::graph::{CompleteProofDag, ProofDagTacticNode},
    theme::Theme,
};

const BRANCH_MID: &str = "├─ ";
const BRANCH_END: &str = "╰─ ";
const VERT_LINE: &str = "│  ";
const EMPTY: &str = "   ";

pub fn build_step_lines(dag: &CompleteProofDag) -> Vec<Line<'static>> {
    let mut active_levels: Vec<bool> = Vec::new();
    let mut lines = Vec::new();

    for node in dag.dfs_iter() {
        let is_current = dag.is_current(node.id);

        let is_last_child = node
            .parent
            .and_then(|pid| dag.get(pid))
            .is_none_or(|parent| parent.children.last() == Some(&node.id));

        let depth = node.depth as usize;
        update_active_levels(&mut active_levels, depth, is_last_child);

        let prefix = build_tree_prefix(&active_levels, depth, is_last_child);
        lines.push(step_line(node, &prefix, is_current));

        let cont_prefix = build_continuation_prefix(&active_levels, depth, is_last_child);

        lines.extend(dependency_line(node, &cont_prefix));
        lines.extend(theorem_line(node, &cont_prefix));
    }

    lines
}

fn step_line_count(node: &ProofDagTacticNode) -> usize {
    1 + usize::from(!node.hypothesis_dependencies.is_empty())
        + usize::from(!node.referenced_theorems.is_empty())
}

fn centered_vertical_scroll(dag: &CompleteProofDag, viewport_height: usize) -> usize {
    let current_line: usize = dag
        .dfs_iter()
        .take_while(|node| !dag.is_current(node.id))
        .map(step_line_count)
        .sum();
    let total_lines: usize = dag.dfs_iter().map(step_line_count).sum();
    let max_scroll = total_lines.saturating_sub(viewport_height);

    current_line
        .saturating_sub(viewport_height / 2)
        .min(max_scroll)
}

pub fn max_step_line_width(dag: &CompleteProofDag) -> usize {
    build_step_lines(dag)
        .iter()
        .map(Line::width)
        .max()
        .unwrap_or(0)
}

pub fn required_sidebar_width(dag: &CompleteProofDag) -> u16 {
    (max_step_line_width(dag) as u16).saturating_add(4)
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

fn build_continuation_prefix(active_levels: &[bool], depth: usize, is_last_child: bool) -> String {
    if depth == 0 {
        return String::from("   ");
    }

    let mut prefix = String::new();

    for &active in active_levels.iter().take(depth.saturating_sub(1)) {
        prefix.push_str(if active { VERT_LINE } else { EMPTY });
    }

    prefix.push_str(if is_last_child { EMPTY } else { VERT_LINE });

    prefix
}

pub struct ProofStepsSidebar<'a> {
    pub dag: &'a CompleteProofDag,
    pub horizontal_scroll: usize,
}

impl Widget for ProofStepsSidebar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::new().fg(Theme::INACTIVE_NODE_BORDER))
            .title(" Steps ")
            .title_style(Style::new().fg(Theme::TITLE_TACTIC));

        let inner = block.inner(area);
        block.render(area, buf);

        let lines = build_step_lines(self.dag);
        let total_lines = lines.len();
        let viewport_height = inner.height as usize;
        let max_content_width = lines.iter().map(Line::width).max().unwrap_or(0);

        let vertical_scroll = centered_vertical_scroll(self.dag, viewport_height)
            .min(total_lines.saturating_sub(viewport_height));
        let horizontal_scroll = self
            .horizontal_scroll
            .min(max_content_width.saturating_sub(inner.width as usize));

        Paragraph::new(lines)
            .scroll((vertical_scroll as u16, horizontal_scroll as u16))
            .render(inner, buf);

        if total_lines > viewport_height && inner.width > 1 {
            let scrollbar_area = Rect {
                x: area.x + area.width.saturating_sub(1),
                y: area.y + 1,
                width: 1,
                height: area.height.saturating_sub(2),
            };
            Scrollbar::new(ScrollbarOrientation::VerticalRight).render(
                scrollbar_area,
                buf,
                &mut ScrollbarState::new(total_lines).position(vertical_scroll),
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
        Span::styled(node.tactic_text.clone(), style),
    ])
}

fn dependency_line(node: &ProofDagTacticNode, prefix: &str) -> Option<Line<'static>> {
    if node.hypothesis_dependencies.is_empty() {
        return None;
    }

    Some(Line::from(vec![
        Span::raw("  "),
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
        Span::raw("  "),
        Span::styled(prefix.to_string(), Theme::INACTIVE_NODE_BORDER),
        Span::styled(
            format!("thms: {}", node.referenced_theorems.join(", ")),
            Style::new().fg(Theme::HYP_NAME).add_modifier(Modifier::DIM),
        ),
    ]))
}
