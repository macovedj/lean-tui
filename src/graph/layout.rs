use petgraph::graph::NodeIndex;

use super::types::{DagGraph, NodeId};

pub const ORPHAN_GAP: i32 = 4;

#[derive(Debug, Clone, Copy)]
pub struct NodePosition {
    pub index: NodeIndex,
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
    pub h_gap: i32,
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
    pub fn find(&self, idx: NodeIndex) -> Option<&NodePosition> {
        self.nodes.iter().find(|n| n.index == idx)
    }
}

pub fn calculate_layout<N, F>(dag: &DagGraph<N>, config: LayoutConfig, size_fn: F) -> DagLayout
where
    F: Fn(&N) -> (u16, u16),
{
    let Some(root) = dag.root else {
        return DagLayout::default();
    };

    let subtree_size = |idx: NodeIndex| -> (i32, i32) {
        subtree_size_recursive(dag, idx, &config, &size_fn)
    };

    let (content_width, content_height) = subtree_size(root);

    let main_nodes = position_subtree(dag, root, 0, 0, content_width, content_height, &config, &size_fn);

    let (orphan_nodes, final_width, final_height) =
        position_orphans(dag, content_width, content_height, &size_fn);

    DagLayout {
        nodes: main_nodes.into_iter().chain(orphan_nodes).collect(),
        content_width: final_width,
        content_height: final_height,
    }
}

fn subtree_size_recursive<N, F>(
    dag: &DagGraph<N>,
    idx: NodeIndex,
    config: &LayoutConfig,
    size_fn: &F,
) -> (i32, i32)
where
    F: Fn(&N) -> (u16, u16),
{
    let Some(node) = dag.get(idx) else {
        return (0, 0);
    };

    let (node_w, node_h) = size_fn(node);
    let (node_w, node_h) = (i32::from(node_w), i32::from(node_h));

    let children = dag.children(idx);

    if children.is_empty() {
        (node_w, node_h)
    } else if children.len() == 1 {
        let (child_w, child_h) = subtree_size_recursive(dag, children[0], config, size_fn);
        (node_w.max(child_w), node_h + config.v_gap + child_h)
    } else {
        let child_sizes: Vec<_> = children
            .iter()
            .map(|&cid| subtree_size_recursive(dag, cid, config, size_fn))
            .collect();
        let total_w: i32 = child_sizes.iter().map(|(w, _)| w).sum();
        let max_child_h = child_sizes.iter().map(|(_, h)| h).max().copied().unwrap_or(0);
        let gaps = config.h_gap * (children.len() as i32 - 1).max(0);

        ((total_w + gaps).max(node_w), node_h + config.v_gap + max_child_h)
    }
}

#[allow(clippy::too_many_arguments)]
fn position_subtree<N, F>(
    dag: &DagGraph<N>,
    idx: NodeIndex,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    config: &LayoutConfig,
    size_fn: &F,
) -> Vec<NodePosition>
where
    F: Fn(&N) -> (u16, u16),
{
    let Some(node) = dag.get(idx) else {
        return Vec::new();
    };

    let (node_w, node_h) = size_fn(node);
    let box_h = i32::from(node_h);
    let (subtree_w, _) = subtree_size_recursive(dag, idx, config, size_fn);

    let node_y = if config.top_down { y } else { y + h - box_h };

    let width = if config.stretch_children {
        u16::try_from(w).unwrap_or(node_w).max(node_w)
    } else {
        node_w
    };

    let current = NodePosition {
        index: idx,
        node_id: dag.node_id(idx).unwrap_or(0),
        x,
        y: node_y,
        width,
        height: node_h,
    };

    let children = dag.children(idx);
    if children.is_empty() {
        return vec![current];
    }

    let child_y = if config.top_down {
        y + box_h + config.v_gap
    } else {
        y - config.v_gap
    };
    let child_h = h - box_h - config.v_gap;

    let child_positions = if children.len() == 1 {
        let (child_w, child_subtree_h) = subtree_size_recursive(dag, children[0], config, size_fn);
        position_subtree(dag, children[0], x, child_y, child_w, child_h.min(child_subtree_h), config, size_fn)
    } else {
        position_multiple_children(dag, &children, x, child_y, child_h, subtree_w, config, size_fn)
    };

    std::iter::once(current).chain(child_positions).collect()
}

#[allow(clippy::too_many_arguments)]
fn position_multiple_children<N, F>(
    dag: &DagGraph<N>,
    children: &[NodeIndex],
    x: i32,
    y: i32,
    h: i32,
    subtree_w: i32,
    config: &LayoutConfig,
    size_fn: &F,
) -> Vec<NodePosition>
where
    F: Fn(&N) -> (u16, u16),
{
    let children_with_sizes: Vec<_> = children
        .iter()
        .map(|&cid| (cid, subtree_size_recursive(dag, cid, config, size_fn)))
        .collect();

    if config.stretch_children {
        let num_children = children.len() as i32;
        let per_child = subtree_w / num_children;
        let remainder = subtree_w % num_children;

        children_with_sizes
            .iter()
            .enumerate()
            .scan(x, |cx, (i, &(cid, (_, ch)))| {
                let cw = per_child + i32::from((i as i32) < remainder);
                let result = position_subtree(dag, cid, *cx, y, cw, h.min(ch), config, size_fn);
                *cx += cw;
                Some(result)
            })
            .flatten()
            .collect()
    } else {
        let total_children_w: i32 = children_with_sizes.iter().map(|(_, (w, _))| w).sum();
        let gaps = config.h_gap * (children.len() as i32 - 1).max(0);
        let start_x = x + (subtree_w - total_children_w - gaps) / 2;

        children_with_sizes
            .iter()
            .scan(start_x, |cx, &(cid, (cw, ch))| {
                let result = position_subtree(dag, cid, *cx, y, cw, h.min(ch), config, size_fn);
                *cx += cw + config.h_gap;
                Some(result)
            })
            .flatten()
            .collect()
    }
}

fn position_orphans<N, F>(
    dag: &DagGraph<N>,
    content_width: i32,
    content_height: i32,
    size_fn: &F,
) -> (Vec<NodePosition>, i32, i32)
where
    F: Fn(&N) -> (u16, u16),
{
    if dag.orphans.is_empty() {
        return (Vec::new(), content_width, content_height);
    }

    let orphan_x = content_width + ORPHAN_GAP;

    let (positions, final_y, max_width) = dag.orphans.iter().fold(
        (Vec::new(), 0i32, content_width),
        |(mut positions, y, max_w), &idx| {
            let Some(node) = dag.get(idx) else {
                return (positions, y, max_w);
            };
            let (node_w, node_h) = size_fn(node);

            positions.push(NodePosition {
                index: idx,
                node_id: dag.node_id(idx).unwrap_or(0),
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
