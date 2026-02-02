//! Navigation region building for functional tableau.

use tracing::trace;

use crate::{
    protocol::{CompleteFunctionalDag, FunctionalDagNode},
    widgets::{
        tableau::{NavigationRegion, NodePosition, TreeLayout},
        Selection,
    },
};

/// Build navigation regions for tree nodes only.
pub fn build_navigation_regions(
    dag: &CompleteFunctionalDag,
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

            let binding_regions = build_binding_regions(pos, node);
            let goal_regions = build_goal_regions(pos, node);

            binding_regions.into_iter().chain(goal_regions)
        })
        .collect();

    trace!(total_regions = regions.len(), "Navigation regions built");
    regions
}

/// Build navigation regions for bindings in a node.
fn build_binding_regions(pos: &NodePosition, node: &FunctionalDagNode) -> Vec<NavigationRegion> {
    const BINDING_WIDTH: u16 = 15;

    node.new_binding_indices
        .iter()
        .enumerate()
        .map(|(i, &binding_idx)| {
            let binding_offset = (i as i32) * (i32::from(BINDING_WIDTH) + 1);
            NavigationRegion {
                x: pos.x + binding_offset,
                y: pos.y,
                width: BINDING_WIDTH,
                height: 1,
                selection: Selection::Hyp {
                    node_id: pos.node_id,
                    hyp_idx: binding_idx as usize,
                },
            }
        })
        .collect()
}

/// Build navigation regions for expected type (goal) in a node.
fn build_goal_regions(pos: &NodePosition, node: &FunctionalDagNode) -> Vec<NavigationRegion> {
    // Each functional node has at most one "goal" (expected type)
    if node.expected_type.is_none() && !node.children.is_empty() {
        // Internal node without expected type - no selectable goal
        return Vec::new();
    }

    vec![NavigationRegion {
        x: pos.x + 1,
        y: pos.y + i32::from(pos.height.saturating_sub(2)),
        width: pos.width.saturating_sub(2),
        height: 1,
        selection: Selection::Goal {
            node_id: pos.node_id,
            goal_idx: 0,
        },
    }]
}
