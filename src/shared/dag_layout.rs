use std::iter::once;

use tracing::{debug, trace};

use crate::protocol::{Dag, DagNode, NodeId};

pub const ORPHAN_GAP: i32 = 4;

#[derive(Debug, Clone, Copy)]
pub struct NodePosition {
    pub node_id: NodeId,
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
}

#[derive(Clone, Copy)]
pub struct LayoutConfig {
    pub top_down: bool,
    pub stretch_children: bool,
    /// Horizontal gap between sibling nodes
    pub h_gap: i32,
    /// Vertical gap between parent and children
    pub v_gap: i32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            top_down: false,
            stretch_children: false,
            h_gap: 2,
            v_gap: 2,
        }
    }
}

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

/// Trait for computing node layout size. Node types implement this directly.
pub trait LayoutSize {
    fn layout_size(&self) -> (u16, u16);
}

#[derive(Clone, Copy)]
struct Rect {
    x: i32,
    y: i32,
    w: i32,
    h: i32,
}

struct LayoutContext<'a, D: Dag>
where
    D::Node: LayoutSize,
{
    dag: &'a D,
    config: LayoutConfig,
}

impl<D: Dag> LayoutContext<'_, D>
where
    D::Node: LayoutSize,
{
    fn subtree_size(&self, node_id: NodeId) -> (i32, i32) {
        self.dag.get_node(node_id).map_or((0, 0), |node| {
            let (node_w, node_h) = node.layout_size();
            let (node_w, node_h) = (i32::from(node_w), i32::from(node_h));

            let children = node.children();
            let result = if children.is_empty() {
                (node_w, node_h)
            } else if children.len() == 1 {
                // Single child: vertical layout - keep max width, sum heights + v_gap
                let (child_w, child_h) = self.subtree_size(children[0]);
                (node_w.max(child_w), node_h + self.config.v_gap + child_h)
            } else {
                // Multiple children: horizontal layout - sum widths + h_gaps, keep max height
                let child_sizes: Vec<_> =
                    children.iter().map(|&cid| self.subtree_size(cid)).collect();
                let total_w: i32 = child_sizes.iter().map(|(w, _)| w).sum();
                let max_child_h = child_sizes
                    .iter()
                    .map(|(_, h)| h)
                    .max()
                    .copied()
                    .unwrap_or(0);
                let gaps = self.config.h_gap * (children.len() as i32 - 1).max(0);

                (
                    (total_w + gaps).max(node_w),
                    node_h + self.config.v_gap + max_child_h,
                )
            };
            trace!(
                "subtree_size: node={}, children={}, size=({}, {})",
                node_id,
                children.len(),
                result.0,
                result.1
            );
            result
        })
    }

    fn position_subtree(&self, node_id: NodeId, rect: Rect) -> Vec<NodePosition> {
        self.dag.get_node(node_id).map_or_else(Vec::new, |node| {
            let (node_w, node_h) = node.layout_size();
            let box_h = i32::from(node_h);
            let (subtree_w, _) = self.subtree_size(node_id);

            let node_y = if self.config.top_down {
                rect.y
            } else {
                rect.y + rect.h - box_h
            };

            let width = if self.config.stretch_children {
                u16::try_from(rect.w).unwrap_or(node_w).max(node_w)
            } else {
                node_w
            };

            let current = NodePosition {
                node_id,
                x: rect.x,
                y: node_y,
                width,
                height: node_h,
            };

            trace!(
                "position_subtree: node={}, rect=({},{},{},{}), pos=({},{})",
                node_id,
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                current.x,
                current.y
            );

            let child_positions = self.position_children(node, rect, box_h, subtree_w);

            once(current).chain(child_positions).collect()
        })
    }

    fn position_children(
        &self,
        node: &D::Node,
        parent_rect: Rect,
        box_h: i32,
        subtree_w: i32,
    ) -> Vec<NodePosition> {
        let children = node.children();
        if children.is_empty() {
            return Vec::new();
        }

        // Add vertical gap between parent and children
        let child_y = if self.config.top_down {
            parent_rect.y + box_h + self.config.v_gap
        } else {
            parent_rect.y - self.config.v_gap
        };
        let child_h = parent_rect.h - box_h - self.config.v_gap;

        debug!(
            "position_children: parent={}, children={:?}, parent_rect=({},{},{},{}), child_y={}",
            node.id(),
            children,
            parent_rect.x,
            parent_rect.y,
            parent_rect.w,
            parent_rect.h,
            child_y
        );

        // Single child: position directly below parent (vertical alignment)
        if children.len() == 1 {
            let child_id = children[0];
            let (child_w, child_subtree_h) = self.subtree_size(child_id);
            let rect = Rect {
                x: parent_rect.x,
                y: child_y,
                w: child_w,
                h: child_h.min(child_subtree_h),
            };
            debug!(
                "  single child: child_id={}, rect=({},{},{},{})",
                child_id, rect.x, rect.y, rect.w, rect.h
            );
            return self.position_subtree(child_id, rect);
        }

        // Multiple children: horizontal spread
        let children_with_sizes: Vec<_> = children
            .iter()
            .map(|&cid| (cid, self.subtree_size(cid)))
            .collect();

        debug!("  multiple children: sizes={:?}", children_with_sizes);

        if self.config.stretch_children {
            self.position_stretched(
                &children_with_sizes,
                parent_rect.x,
                child_y,
                parent_rect.w,
                child_h,
            )
        } else {
            let total_children_w: i32 = children_with_sizes.iter().map(|(_, (w, _))| w).sum();
            let gaps = self.config.h_gap * (children.len() as i32 - 1).max(0);
            let start_x = parent_rect.x + (subtree_w - total_children_w - gaps) / 2;
            self.position_centered(&children_with_sizes, start_x, child_y, child_h)
        }
    }

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

    fn position_centered(
        &self,
        children: &[(NodeId, (i32, i32))],
        x: i32,
        y: i32,
        h: i32,
    ) -> Vec<NodePosition> {
        let h_gap = self.config.h_gap;
        children
            .iter()
            .scan(x, |cx, &(cid, (cw, ch))| {
                let rect = Rect {
                    x: *cx,
                    y,
                    w: cw,
                    h: h.min(ch),
                };
                *cx += cw + h_gap;
                Some(self.position_subtree(cid, rect))
            })
            .flatten()
            .collect()
    }

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
                    let (node_w, node_h) = node.layout_size();

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

        (positions, max_width, content_height.max(final_y))
    }
}

pub fn calculate_dag_layout<D>(dag: &D, config: LayoutConfig) -> DagLayout
where
    D: Dag,
    D::Node: LayoutSize,
{
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
