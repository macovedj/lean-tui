//! Navigation region building for semantic tableau.

use tracing::trace;

use crate::{
    protocol::{CompleteProofDag, ProofDagTacticNode},
    widgets::{
        tableau::{NavigationRegion, NodePosition, TreeLayout},
        Selection,
    },
};

/// Build navigation regions for tree nodes only.
pub fn build_navigation_regions(
    dag: &CompleteProofDag,
    layout: &TreeLayout,
) -> Vec<NavigationRegion> {
    trace!(
        node_count = layout.nodes.len(),
        "Building navigation regions"
    );

    let regions: Vec<_> = layout
        .nodes
        .iter()
        .filter_map(|pos| dag.get(pos.node_id).map(|node| (pos, node)))
        .flat_map(|(pos, node)| {
            trace!(
                node_id = pos.node_id,
                x = pos.x,
                y = pos.y,
                "Processing node"
            );

            let hypothesis_regions = build_hypothesis_regions(pos, node);
            let goal_regions = build_goal_regions(pos, node);

            hypothesis_regions.into_iter().chain(goal_regions)
        })
        .collect();

    trace!(total_regions = regions.len(), "Navigation regions built");
    regions
}

/// Build navigation regions for hypotheses in a node.
fn build_hypothesis_regions(
    pos: &NodePosition,
    node: &ProofDagTacticNode,
) -> Vec<NavigationRegion> {
    const HYP_WIDTH: u16 = 15;

    node.new_hypothesis_indices
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
fn build_goal_regions(pos: &NodePosition, node: &ProofDagTacticNode) -> Vec<NavigationRegion> {
    let goal_count = node.proof_state_after.goals.len();
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
