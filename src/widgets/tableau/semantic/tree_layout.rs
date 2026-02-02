//! Tree layout calculation for the semantic tableau.

use std::iter::once;

use crate::{
    protocol::{CompleteProofDag, NodeId, ProofDagTacticNode},
    widgets::tableau::{NodePosition, TreeLayout, ORPHAN_GAP},
};

/// Calculate height for a node box based on content.
pub const fn node_height(node: &ProofDagTacticNode) -> u16 {
    // Border (2) + goals line (1) + optional hypotheses line (1)
    3 + (!node.new_hypothesis_indices.is_empty()) as u16
}

/// Calculate minimum width needed for a node's content.
pub fn node_content_width(node: &ProofDagTacticNode) -> u16 {
    let title_width = node.tactic.text.len() + 8;

    let hypothesis_widths = node.new_hypothesis_indices.iter().filter_map(|&idx| {
        node.proof_state_after
            .hypotheses
            .get(idx as usize)
            .map(|h| h.name.len() + h.type_.to_plain_text().len() + 5)
    });

    let goal_widths = if node.proof_state_after.goals.is_empty() {
        vec![16] // "✓ Goal completed"
    } else {
        node.proof_state_after
            .goals
            .iter()
            .map(|g| g.type_.to_plain_text().len() + 4)
            .collect()
    };

    let max_width = once(title_width)
        .chain(hypothesis_widths)
        .chain(goal_widths)
        .max()
        .unwrap_or(0);

    (max_width + 4) as u16
}

/// Calculate subtree dimensions (width, height).
fn subtree_size(dag: &CompleteProofDag, node_id: NodeId) -> (i32, i32) {
    dag.get(node_id).map_or((0, 0), |node| {
        let node_h = i32::from(node_height(node));
        let node_w = i32::from(node_content_width(node));

        if node.children.is_empty() {
            (node_w, node_h)
        } else {
            let (total_w, max_child_h) = node
                .children
                .iter()
                .map(|&cid| subtree_size(dag, cid))
                .fold((0, 0), |(tw, mh), (cw, ch)| (tw + cw, mh.max(ch)));

            (total_w.max(node_w), node_h + max_child_h)
        }
    })
}

/// Calculate tree layout with actual content dimensions.
pub fn calculate_tree_layout(dag: &CompleteProofDag, top_down: bool) -> TreeLayout {
    let Some(root_id) = dag.root_node_id else {
        return TreeLayout::default();
    };

    let (content_width, content_height) = subtree_size(dag, root_id);

    let main_nodes = position_subtree(dag, root_id, 0, 0, content_height, top_down);

    let (orphan_nodes, final_width, final_height) =
        position_orphans(dag, content_width, content_height);

    TreeLayout {
        nodes: main_nodes.into_iter().chain(orphan_nodes).collect(),
        content_width: final_width,
        content_height: final_height,
    }
}

/// Position all nodes in a subtree recursively, returning positioned nodes.
fn position_subtree(
    dag: &CompleteProofDag,
    node_id: NodeId,
    x: i32,
    y: i32,
    available_h: i32,
    top_down: bool,
) -> Vec<NodePosition> {
    dag.get(node_id).map_or_else(Vec::new, |node| {
        let box_h = i32::from(node_height(node));
        let (subtree_w, _) = subtree_size(dag, node_id);
        let node_w = node_content_width(node);

        let node_y = if top_down { y } else { y + available_h - box_h };

        let current = NodePosition {
            node_id,
            x,
            y: node_y,
            width: u16::try_from(subtree_w).unwrap_or(node_w).max(node_w),
            height: node_height(node),
        };

        let child_positions = if node.children.is_empty() {
            Vec::new()
        } else {
            let child_h = available_h - box_h;
            let child_y = if top_down { y + box_h } else { y };

            node.children
                .iter()
                .scan(x, |cx, &cid| {
                    let (cw, ch) = subtree_size(dag, cid);
                    let positions =
                        position_subtree(dag, cid, *cx, child_y, child_h.min(ch), top_down);
                    *cx += cw;
                    Some(positions)
                })
                .flatten()
                .collect()
        };

        once(current).chain(child_positions).collect()
    })
}

/// Position orphan nodes to the right of the main tree.
fn position_orphans(
    dag: &CompleteProofDag,
    content_width: i32,
    content_height: i32,
) -> (Vec<NodePosition>, i32, i32) {
    if dag.orphans.is_empty() {
        return (Vec::new(), content_width, content_height);
    }

    let orphan_x = content_width + ORPHAN_GAP;

    let (positions, final_y, max_width) = dag.orphans.iter().filter_map(|&id| dag.get(id)).fold(
        (Vec::new(), 0i32, content_width),
        |(mut positions, y, max_w), node| {
            let node_w = node_content_width(node);
            let node_h = node_height(node);

            positions.push(NodePosition {
                node_id: node.id,
                x: orphan_x,
                y,
                width: node_w,
                height: node_h,
            });

            let new_y = y + i32::from(node_h) + 1;
            let new_max_w = max_w.max(orphan_x + i32::from(node_w));

            (positions, new_y, new_max_w)
        },
    );

    let final_height = content_height.max(final_y);

    (positions, max_width, final_height)
}
