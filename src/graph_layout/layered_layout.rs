use std::{
    cmp::Reverse,
    collections::{HashMap, HashSet, VecDeque},
};

use petgraph::graph::NodeIndex;

use crate::protocol::NodeId;

pub const ORPHAN_GAP: i32 = 3;
const VERTICAL_GAP: i32 = 2;
const HORIZONTAL_GAP: i32 = 2;

// Node size calculation constants
const BORDER_PADDING: usize = 6;
const BORDER_HEIGHT: u16 = 2;

/// Trait for nodes that can describe their text content for layout sizing.
pub trait NodeTextContent {
    /// The main title/header text
    fn title(&self) -> String;

    /// Additional content lines (each line's text)
    fn content_lines(&self) -> Vec<String>;
}

/// Calculate layout size from a title and pre-computed content lines.
pub fn calculate_size_from_lines(title: &str, lines: &[String]) -> (u16, u16) {
    let title_width = title.chars().count();
    let max_line_width = lines.iter().map(|s| s.chars().count()).max().unwrap_or(0);
    let max_content_width = title_width.max(max_line_width);

    let width = (max_content_width + BORDER_PADDING) as u16;
    let height = BORDER_HEIGHT + lines.len().max(1) as u16;

    (width, height)
}

/// Calculate layout size for any node with text content.
/// Width is based on the widest line (title or content), height on line count.
pub fn calculate_node_size<T: NodeTextContent>(node: &T) -> (u16, u16) {
    calculate_size_from_lines(&node.title(), &node.content_lines())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodePosition {
    pub index: NodeIndex,
    pub node_id: NodeId,
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Default, PartialEq, Eq)]
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

#[derive(Debug, Clone)]
pub struct LayoutNode {
    pub index: NodeIndex,
    pub id: NodeId,
    pub width: u16,
    pub height: u16,
}

#[derive(Debug, Clone)]
pub struct LayoutEdge {
    pub from: NodeIndex,
    pub to: NodeIndex,
    pub strength: u64,
}

#[derive(Debug, Clone, Default)]
pub struct LayoutGraph {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    pub root: Option<NodeIndex>,
    pub orphans: Vec<NodeIndex>,
}

impl LayoutGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, index: NodeIndex, id: NodeId, width: u16, height: u16) {
        self.nodes.push(LayoutNode {
            index,
            id,
            width,
            height,
        });
    }

    pub fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, strength: u64) {
        self.edges.push(LayoutEdge { from, to, strength });
    }
}

pub fn calculate_layout(graph: &LayoutGraph, top_down: bool) -> DagLayout {
    if graph.nodes.is_empty() {
        return DagLayout::default();
    }

    let orphan_set: HashSet<_> = graph.orphans.iter().copied().collect();
    let (main_nodes, orphan_nodes): (Vec<_>, Vec<_>) = graph
        .nodes
        .iter()
        .partition(|n| !orphan_set.contains(&n.index));

    if main_nodes.is_empty() {
        let (positions, width, height) = layout_orphans_only(&orphan_nodes);
        return DagLayout {
            nodes: positions,
            content_width: width,
            content_height: height,
        };
    }

    let main_positions = layered_layout(&main_nodes, &graph.edges, graph.root, top_down);
    let (orphan_positions, final_width, final_height) = position_orphans(
        &orphan_nodes,
        main_positions.content_width,
        main_positions.content_height,
    );

    DagLayout {
        nodes: main_positions
            .nodes
            .into_iter()
            .chain(orphan_positions)
            .collect(),
        content_width: final_width,
        content_height: final_height,
    }
}

fn layered_layout(
    nodes: &[&LayoutNode],
    edges: &[LayoutEdge],
    root: Option<NodeIndex>,
    top_down: bool,
) -> DagLayout {
    if nodes.is_empty() {
        return DagLayout::default();
    }

    let mut children: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
    for edge in edges {
        children.entry(edge.from).or_default().push(edge.to);
    }

    let depths = calculate_depths(nodes, &children, root);

    let max_depth = depths.values().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<&LayoutNode>> = vec![Vec::new(); max_depth + 1];
    for node in nodes {
        let depth = depths.get(&node.index).copied().unwrap_or(0);
        layers[depth].push(node);
    }

    let mut layer_y_positions: Vec<i32> = Vec::with_capacity(layers.len());
    let mut current_y = 0i32;
    for layer in &layers {
        layer_y_positions.push(current_y);
        let max_height = layer.iter().map(|n| i32::from(n.height)).max().unwrap_or(1);
        current_y += max_height + VERTICAL_GAP;
    }

    let x_positions = hierarchical_x_positions(nodes, edges, &depths, root);

    let positions: Vec<NodePosition> = nodes
        .iter()
        .map(|node| {
            let depth = depths.get(&node.index).copied().unwrap_or(0);
            let y = if top_down {
                layer_y_positions[depth]
            } else {
                current_y - layer_y_positions[depth] - i32::from(node.height)
            };
            let x = x_positions.get(&node.index).copied().unwrap_or(0);

            NodePosition {
                index: node.index,
                node_id: node.id,
                x,
                y,
                width: node.width,
                height: node.height,
            }
        })
        .collect();

    let content_width = positions
        .iter()
        .map(|n| n.x + i32::from(n.width))
        .max()
        .unwrap_or(0);
    let content_height = positions
        .iter()
        .map(|n| n.y + i32::from(n.height))
        .max()
        .unwrap_or(0);

    DagLayout {
        nodes: positions,
        content_width,
        content_height,
    }
}

fn calculate_depths(
    nodes: &[&LayoutNode],
    children: &HashMap<NodeIndex, Vec<NodeIndex>>,
    root: Option<NodeIndex>,
) -> HashMap<NodeIndex, usize> {
    let mut depths: HashMap<NodeIndex, usize> = HashMap::new();
    let node_set: HashSet<_> = nodes.iter().map(|n| n.index).collect();

    let root_idx = root.or_else(|| {
        let has_parent: HashSet<_> = children.values().flatten().copied().collect();
        nodes
            .iter()
            .find(|n| !has_parent.contains(&n.index))
            .map(|n| n.index)
    });

    if let Some(root) = root_idx {
        bfs_depths(root, children, &node_set, &mut depths);
    }

    for node in nodes {
        depths.entry(node.index).or_insert(0);
    }

    depths
}

fn bfs_depths(
    root: NodeIndex,
    children: &HashMap<NodeIndex, Vec<NodeIndex>>,
    node_set: &HashSet<NodeIndex>,
    depths: &mut HashMap<NodeIndex, usize>,
) {
    let mut queue = VecDeque::new();
    queue.push_back((root, 0usize));

    while let Some((idx, depth)) = queue.pop_front() {
        if depths.contains_key(&idx) {
            continue;
        }
        depths.insert(idx, depth);

        let Some(child_list) = children.get(&idx) else {
            continue;
        };
        for &child in child_list {
            if node_set.contains(&child) && !depths.contains_key(&child) {
                queue.push_back((child, depth + 1));
            }
        }
    }
}

fn hierarchical_x_positions(
    nodes: &[&LayoutNode],
    edges: &[LayoutEdge],
    depths: &HashMap<NodeIndex, usize>,
    root: Option<NodeIndex>,
) -> HashMap<NodeIndex, i32> {
    if nodes.is_empty() {
        return HashMap::new();
    }

    let mut children_with_strength: HashMap<NodeIndex, Vec<(NodeIndex, u64)>> = HashMap::new();
    for edge in edges {
        children_with_strength
            .entry(edge.from)
            .or_default()
            .push((edge.to, edge.strength));
    }

    let children: HashMap<NodeIndex, Vec<NodeIndex>> = children_with_strength
        .into_iter()
        .map(|(parent, mut kids)| {
            kids.sort_by_key(|k| Reverse(k.1));
            let sorted_indices: Vec<NodeIndex> = kids.into_iter().map(|(idx, _)| idx).collect();
            (parent, sorted_indices)
        })
        .collect();

    let widths: HashMap<NodeIndex, i32> = nodes
        .iter()
        .map(|n| (n.index, i32::from(n.width)))
        .collect();

    let max_depth = depths.values().copied().max().unwrap_or(0);
    let mut layers: Vec<Vec<NodeIndex>> = vec![Vec::new(); max_depth + 1];
    for node in nodes {
        let depth = depths.get(&node.index).copied().unwrap_or(0);
        layers[depth].push(node.index);
    }

    let mut x_positions: HashMap<NodeIndex, i32> = HashMap::new();

    if let Some(root_idx) = root.or_else(|| layers.first().and_then(|l| l.first().copied())) {
        x_positions.insert(root_idx, 0);
    }

    for layer in layers.iter().take(max_depth + 1) {
        for &node_idx in layer {
            let Some(node_children) = children.get(&node_idx) else {
                continue;
            };
            let parent_x = x_positions.get(&node_idx).copied().unwrap_or(0);
            let parent_width = widths.get(&node_idx).copied().unwrap_or(0);
            let parent_center = parent_x + parent_width / 2;

            position_children(node_children, parent_center, &widths, &mut x_positions);
        }
        resolve_layer_overlaps(layer, &widths, &mut x_positions);
    }

    let min_x = x_positions.values().copied().min().unwrap_or(0);
    if min_x < 0 {
        for x in x_positions.values_mut() {
            *x -= min_x;
        }
    }

    x_positions
}

/// Interleave children in centered order (alternating left/right from center).
fn interleave_children(children: &[NodeIndex]) -> Vec<NodeIndex> {
    let mut left = Vec::new();
    let mut right = Vec::new();
    for (i, &child) in children.iter().enumerate() {
        if i % 2 == 0 {
            right.push(child);
        } else {
            left.push(child);
        }
    }
    left.reverse();
    left.into_iter().chain(right).collect()
}

/// Position children centered under their parent.
fn position_children(
    node_children: &[NodeIndex],
    parent_center: i32,
    widths: &HashMap<NodeIndex, i32>,
    x_positions: &mut HashMap<NodeIndex, i32>,
) {
    let total_children_width: i32 = node_children
        .iter()
        .filter_map(|c| widths.get(c))
        .map(|w| w + HORIZONTAL_GAP)
        .sum::<i32>()
        .saturating_sub(HORIZONTAL_GAP);

    let centered_children = interleave_children(node_children);
    let mut child_x = parent_center - total_children_width / 2;

    for child_idx in centered_children {
        let child_width = widths.get(&child_idx).copied().unwrap_or(0);
        x_positions.insert(child_idx, child_x);
        child_x += child_width + HORIZONTAL_GAP;
    }
}

fn resolve_layer_overlaps(
    layer: &[NodeIndex],
    widths: &HashMap<NodeIndex, i32>,
    x_positions: &mut HashMap<NodeIndex, i32>,
) {
    if layer.len() < 2 {
        return;
    }

    let mut sorted: Vec<_> = layer
        .iter()
        .map(|&idx| (idx, x_positions.get(&idx).copied().unwrap_or(0)))
        .collect();
    sorted.sort_by_key(|(_, x)| *x);

    let mut prev_right = i32::MIN;
    for (idx, _) in &sorted {
        let current_x = x_positions.get(idx).copied().unwrap_or(0);
        let width = widths.get(idx).copied().unwrap_or(0);

        let min_x = if prev_right == i32::MIN {
            current_x
        } else {
            prev_right + HORIZONTAL_GAP
        };

        let new_x = current_x.max(min_x);
        x_positions.insert(*idx, new_x);
        prev_right = new_x + width;
    }
}

fn layout_orphans_only(orphans: &[&LayoutNode]) -> (Vec<NodePosition>, i32, i32) {
    if orphans.is_empty() {
        return (Vec::new(), 0, 0);
    }

    let (positions, final_y, max_width) = orphans.iter().fold(
        (Vec::new(), 0i32, 0i32),
        |(mut positions, y, max_w), node| {
            positions.push(NodePosition {
                index: node.index,
                node_id: node.id,
                x: 0,
                y,
                width: node.width,
                height: node.height,
            });

            let new_y = y + i32::from(node.height) + 1;
            let new_max_w = max_w.max(i32::from(node.width));

            (positions, new_y, new_max_w)
        },
    );

    (positions, max_width, final_y)
}

fn position_orphans(
    orphans: &[&LayoutNode],
    content_width: i32,
    content_height: i32,
) -> (Vec<NodePosition>, i32, i32) {
    if orphans.is_empty() {
        return (Vec::new(), content_width, content_height);
    }

    let orphan_x = content_width + ORPHAN_GAP;

    let (positions, final_y, max_width) = orphans.iter().fold(
        (Vec::new(), 0i32, content_width),
        |(mut positions, y, max_w), node| {
            positions.push(NodePosition {
                index: node.index,
                node_id: node.id,
                x: orphan_x,
                y,
                width: node.width,
                height: node.height,
            });

            let new_y = y + i32::from(node.height) + 1;
            let new_max_w = max_w.max(orphan_x + i32::from(node.width));

            (positions, new_y, new_max_w)
        },
    );

    (positions, max_width, content_height.max(final_y))
}
