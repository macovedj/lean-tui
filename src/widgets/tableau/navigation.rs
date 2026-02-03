//! Shared navigation types and algorithms for tableau views.

use tracing::trace;

use super::{DagLayout, NodePosition};
use crate::{
    protocol::{Dag, NavNode},
    widgets::Selection,
};

/// Region for keyboard navigation - uses virtual coordinates (i32).
#[derive(Debug, Clone, Copy)]
pub struct NavigationRegion {
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
    pub selection: Selection,
}

/// Direction for spatial navigation.
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Check if `other` is ahead of `cur` in this direction.
    fn is_ahead(self, cur: &NavigationRegion, other: &NavigationRegion) -> bool {
        match self {
            Self::Left => other.x + i32::from(other.width) <= cur.x,
            Self::Right => other.x >= cur.x + i32::from(cur.width),
            Self::Up => other.y + i32::from(other.height) <= cur.y,
            Self::Down => other.y >= cur.y + i32::from(cur.height),
        }
    }

    /// Calculate distance from `cur` to `other` in this direction.
    fn distance(self, cur: &NavigationRegion, other: &NavigationRegion) -> i32 {
        match self {
            Self::Left => (cur.x - (other.x + i32::from(other.width))).max(0),
            Self::Right => (other.x - (cur.x + i32::from(cur.width))).max(0),
            Self::Up => (cur.y - (other.y + i32::from(other.height))).max(0),
            Self::Down => (other.y - (cur.y + i32::from(cur.height))).max(0),
        }
    }

    /// Whether this is a horizontal direction.
    const fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

/// Check if two 1D ranges overlap.
pub fn ranges_overlap(a_start: i32, a_len: u16, b_start: i32, b_len: u16) -> bool {
    let a_end = a_start + i32::from(a_len);
    let b_end = b_start + i32::from(b_len);
    a_start < b_end && b_start < a_end
}

// --- Generic navigation region building ---

/// Width of hypothesis/binding regions.
const HYP_WIDTH: u16 = 15;

/// Build navigation regions for all nodes in a layout.
pub fn build_navigation_regions<D>(dag: &D, layout: &DagLayout) -> Vec<NavigationRegion>
where
    D: Dag,
    D::Node: NavNode,
{
    trace!(
        node_count = layout.nodes.len(),
        "Building navigation regions"
    );

    let regions: Vec<_> = layout
        .nodes
        .iter()
        .filter_map(|pos| dag.get_node(pos.node_id).map(|node| (pos, node)))
        .flat_map(|(pos, node)| {
            trace!(
                node_id = pos.node_id,
                x = pos.x,
                y = pos.y,
                "Processing node"
            );

            let hyp_regions = build_hyp_regions(pos, node);
            let goal_regions = build_goal_regions(pos, node);

            hyp_regions.into_iter().chain(goal_regions)
        })
        .collect();

    trace!(total_regions = regions.len(), "Navigation regions built");
    regions
}

/// Build navigation regions for hypotheses/bindings in a node.
fn build_hyp_regions<N: NavNode>(pos: &NodePosition, node: &N) -> Vec<NavigationRegion> {
    node.new_hyp_indices()
        .iter()
        .enumerate()
        .map(|(i, &hyp_idx)| {
            let hyp_offset = (i as i32) * (i32::from(HYP_WIDTH) + 1);
            NavigationRegion {
                x: pos.x + hyp_offset,
                y: pos.y,
                width: HYP_WIDTH,
                height: 1,
                selection: Selection::Hyp {
                    node_id: pos.node_id,
                    hyp_idx: hyp_idx as usize,
                },
            }
        })
        .collect()
}

/// Build navigation regions for goals in a node.
fn build_goal_regions<N: NavNode>(pos: &NodePosition, node: &N) -> Vec<NavigationRegion> {
    if !node.has_selectable_goals() {
        return Vec::new();
    }

    let goal_count = node.goal_count();
    if goal_count == 0 {
        return Vec::new();
    }

    let goal_width = if goal_count > 1 {
        (pos.width.saturating_sub(2) / goal_count as u16).max(15)
    } else {
        pos.width.saturating_sub(2)
    };

    (0..goal_count)
        .map(|i| {
            let goal_offset = (i as i32) * (i32::from(goal_width) + 1);
            NavigationRegion {
                x: pos.x + 1 + goal_offset,
                y: pos.y + i32::from(pos.height.saturating_sub(2)),
                width: goal_width,
                height: 1,
                selection: Selection::Goal {
                    node_id: pos.node_id,
                    goal_idx: i,
                },
            }
        })
        .collect()
}

/// Find the nearest selectable item in the given direction.
pub fn find_nearest_in_direction(
    regions: &[NavigationRegion],
    current: Selection,
    direction: Direction,
) -> Option<Selection> {
    let cur = regions.iter().find(|r| r.selection == current)?;

    let is_aligned = |r: &NavigationRegion| {
        let overlaps = if direction.is_horizontal() {
            ranges_overlap(cur.y, cur.height, r.y, r.height)
        } else {
            ranges_overlap(cur.x, cur.width, r.x, r.width)
        };
        overlaps && direction.is_ahead(cur, r)
    };

    regions
        .iter()
        .filter(|r| is_aligned(r))
        .min_by_key(|r| direction.distance(cur, r))
        .map(|r| r.selection)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hyp_region(node_id: u64, hyp_idx: usize, x: i32, y: i32) -> NavigationRegion {
        NavigationRegion {
            x,
            y,
            width: 15,
            height: 1,
            selection: Selection::Hyp { node_id, hyp_idx },
        }
    }

    fn goal_region(node_id: u64, goal_idx: usize, x: i32, y: i32, width: u16) -> NavigationRegion {
        NavigationRegion {
            x,
            y,
            width,
            height: 1,
            selection: Selection::Goal { node_id, goal_idx },
        }
    }

    #[test]
    fn test_sibling_hypothesis_navigation() {
        let regions = vec![
            hyp_region(1, 0, 0, 10),
            hyp_region(1, 1, 16, 10),
            goal_region(1, 0, 1, 12, 30),
        ];

        assert_eq!(
            find_nearest_in_direction(
                &regions,
                Selection::Hyp {
                    node_id: 1,
                    hyp_idx: 0
                },
                Direction::Right
            ),
            Some(Selection::Hyp {
                node_id: 1,
                hyp_idx: 1
            })
        );

        assert_eq!(
            find_nearest_in_direction(
                &regions,
                Selection::Hyp {
                    node_id: 1,
                    hyp_idx: 1
                },
                Direction::Left
            ),
            Some(Selection::Hyp {
                node_id: 1,
                hyp_idx: 0
            })
        );

        assert_eq!(
            find_nearest_in_direction(
                &regions,
                Selection::Hyp {
                    node_id: 1,
                    hyp_idx: 0
                },
                Direction::Down
            ),
            Some(Selection::Goal {
                node_id: 1,
                goal_idx: 0
            })
        );
    }

    #[test]
    fn test_navigation_no_target() {
        let regions = vec![hyp_region(1, 0, 0, 10)];

        assert_eq!(
            find_nearest_in_direction(
                &regions,
                Selection::Hyp {
                    node_id: 1,
                    hyp_idx: 0
                },
                Direction::Right
            ),
            None
        );
    }

    #[test]
    fn test_ranges_overlap() {
        assert!(ranges_overlap(10, 1, 10, 1));
        assert!(!ranges_overlap(10, 1, 11, 1));
        assert!(ranges_overlap(10, 5, 12, 5));
        assert!(ranges_overlap(10, 10, 12, 2));
    }
}
