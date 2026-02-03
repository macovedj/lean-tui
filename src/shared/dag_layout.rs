use std::iter::once;

use crate::protocol::{Dag, DagNode, NodeId};

/// Gap between main tree and orphan nodes.
pub const ORPHAN_GAP: i32 = 4;

/// Position and dimensions of a node in the layout.
#[derive(Debug, Clone, Copy)]
pub struct NodePosition {
    pub node_id: NodeId,
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
}

/// Configuration for DAG layout calculation.
#[derive(Clone, Copy, Default)]
pub struct LayoutConfig {
    /// When true, tree renders top-down (parent above children).
    pub top_down: bool,
    /// When true, children stretch to fill parent width equally.
    pub stretch_children: bool,
}

/// Computed DAG layout with positioned nodes.
#[derive(Debug, Default)]
pub struct DagLayout {
    pub nodes: Vec<NodePosition>,
    pub content_width: i32,
    pub content_height: i32,
}

impl DagLayout {
    pub fn find_node(&self, node_id: NodeId) -> Option<&NodePosition> {
        self.nodes.iter().find(|n| n.node_id == node_id)
    }
}

/// Simple rectangle for layout positioning.
#[derive(Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

/// Context for recursive layout positioning.
struct LayoutContext<'a, D: Dag> {
    dag: &'a D,
    config: LayoutConfig,
}

impl<D: Dag> LayoutContext<'_, D> {
    /// Calculate subtree dimensions (width, height).
    fn subtree_size(&self, node_id: NodeId) -> (i32, i32) {
        self.dag.get_node(node_id).map_or((0, 0), |node| {
            let node_h = i32::from(node.box_height());
            let node_w = i32::from(node.content_width());

            if node.children().is_empty() {
                (node_w, node_h)
            } else {
                let (total_w, max_child_h) = node
                    .children()
                    .iter()
                    .map(|&cid| self.subtree_size(cid))
                    .fold((0, 0), |(tw, mh), (cw, ch)| (tw + cw, mh.max(ch)));

                (total_w.max(node_w), node_h + max_child_h)
            }
        })
    }

    /// Position all nodes in a subtree recursively.
    fn position_subtree(&self, node_id: NodeId, rect: Rect) -> Vec<NodePosition> {
        self.dag.get_node(node_id).map_or_else(Vec::new, |node| {
            let box_h = i32::from(node.box_height());
            let (subtree_w, _) = self.subtree_size(node_id);
            let node_w = node.content_width();

            let node_y = if self.config.top_down {
                rect.y
            } else {
                rect.y + rect.h - box_h
            };

            let current = NodePosition {
                node_id,
                x: rect.x,
                y: node_y,
                width: u16::try_from(rect.w).unwrap_or(node_w).max(node_w),
                height: node.box_height(),
            };

            let child_positions = self.position_children(node, rect, box_h, subtree_w);

            once(current).chain(child_positions).collect()
        })
    }

    /// Position children of a node.
    fn position_children(
        &self,
        node: &D::Node,
        parent_rect: Rect,
        box_h: i32,
        subtree_w: i32,
    ) -> Vec<NodePosition> {
        if node.children().is_empty() {
            return Vec::new();
        }

        let child_y = if self.config.top_down {
            parent_rect.y + box_h
        } else {
            parent_rect.y
        };
        let child_h = parent_rect.h - box_h;

        let children: Vec<_> = node
            .children()
            .iter()
            .map(|&cid| (cid, self.subtree_size(cid)))
            .collect();

        if self.config.stretch_children {
            self.position_stretched(&children, parent_rect.x, child_y, parent_rect.w, child_h)
        } else {
            let total_children_w: i32 = children.iter().map(|(_, (w, _))| w).sum();
            let start_x = parent_rect.x + (subtree_w - total_children_w) / 2;
            self.position_centered(&children, start_x, child_y, child_h)
        }
    }

    /// Position children with equal width stretch to fill parent.
    #[allow(clippy::many_single_char_names)]
    fn position_stretched(
        &self,
        children: &[(NodeId, (i32, i32))],
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> Vec<NodePosition> {
        let num_children = children.len() as i32;
        let per_child = w / num_children;
        let remainder = w % num_children;

        children
            .iter()
            .enumerate()
            .scan(x, |cx, (i, &(cid, (_, ch)))| {
                let cw = per_child + i32::from((i as i32) < remainder);
                let rect = Rect {
                    x: *cx,
                    y,
                    w: cw,
                    h: h.min(ch),
                };
                *cx += cw;
                Some(self.position_subtree(cid, rect))
            })
            .flatten()
            .collect()
    }

    /// Position children centered under parent with natural widths.
    fn position_centered(
        &self,
        children: &[(NodeId, (i32, i32))],
        x: i32,
        y: i32,
        h: i32,
    ) -> Vec<NodePosition> {
        children
            .iter()
            .scan(x, |cx, &(cid, (cw, ch))| {
                let rect = Rect {
                    x: *cx,
                    y,
                    w: cw,
                    h: h.min(ch),
                };
                *cx += cw;
                Some(self.position_subtree(cid, rect))
            })
            .flatten()
            .collect()
    }

    /// Position orphan nodes to the right of the main tree.
    fn position_orphans(
        &self,
        content_width: i32,
        content_height: i32,
    ) -> (Vec<NodePosition>, i32, i32) {
        let orphans = self.dag.orphan_ids();
        if orphans.is_empty() {
            return (Vec::new(), content_width, content_height);
        }

        let orphan_x = content_width + ORPHAN_GAP;

        let (positions, final_y, max_width) =
            orphans.iter().filter_map(|&id| self.dag.get_node(id)).fold(
                (Vec::new(), 0i32, content_width),
                |(mut positions, y, max_w), node| {
                    let node_w = node.content_width();
                    let node_h = node.box_height();

                    positions.push(NodePosition {
                        node_id: node.id(),
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
}

/// Calculate DAG layout with actual content dimensions.
pub fn calculate_dag_layout<D: Dag>(dag: &D, config: LayoutConfig) -> DagLayout {
    let Some(root_id) = dag.root_id() else {
        return DagLayout::default();
    };

    let ctx = LayoutContext { dag, config };
    let (content_width, content_height) = ctx.subtree_size(root_id);

    let rect = Rect {
        x: 0,
        y: 0,
        w: content_width,
        h: content_height,
    };
    let main_nodes = ctx.position_subtree(root_id, rect);

    let (orphan_nodes, final_width, final_height) =
        ctx.position_orphans(content_width, content_height);

    DagLayout {
        nodes: main_nodes.into_iter().chain(orphan_nodes).collect(),
        content_width: final_width,
        content_height: final_height,
    }
}
