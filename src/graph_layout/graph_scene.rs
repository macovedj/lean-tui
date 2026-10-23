use ratatui::{buffer::Buffer, layout::Rect};

use super::{render_edges, render_scrollbars, Dag, DagGraph, DagLayout, NavNode, NodePosition};
use crate::{
    message::{ClickTarget, ModeAction},
    navigation::{build_navigation_regions, find_nearest_in_direction, Direction},
    protocol::NodeId,
    selection::{AnchoredSelection, Selection},
    widgets::node_content::NodeContentWidget,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScrollOffset {
    pub x: i32,
    pub y: i32,
}

pub struct GraphScene<'a, N> {
    pub graph: &'a DagGraph<N>,
    pub layout: &'a DagLayout,
    pub selection: Option<Selection>,
    pub top_down: bool,
}

pub fn draw_graph_scene<N>(
    scene: &GraphScene<'_, N>,
    area: Rect,
    buf: &mut Buffer,
    node_widget: impl Fn(&N, bool) -> NodeContentWidget,
) -> Vec<ClickTarget> {
    if scene.layout.nodes.is_empty() {
        return Vec::new();
    }

    let selected_index = scene.selection.and_then(|selection| match selection {
        Selection::Goal { node_id, .. } | Selection::Hyp { node_id, .. } => {
            scene.graph.index_of(node_id)
        }
        Selection::Error { .. } => None,
    });
    let scroll = selected_index
        .into_iter()
        .chain(scene.graph.current)
        .chain(scene.graph.root)
        .find_map(|index| scene.layout.find(index))
        .map_or_else(ScrollOffset::default, |target| {
            center_on(target, scene.layout, area)
        });

    render_edges(
        scene.graph,
        scene.layout,
        scroll.x,
        scroll.y,
        scene.top_down,
        area,
        buf,
    );

    let click_targets = scene
        .layout
        .nodes
        .iter()
        .filter_map(|position| {
            let node_area = clip_to_viewport(position, scroll, area)
                .filter(|node_area| node_area.width >= 3 && node_area.height >= 3)?;
            let node = scene.graph.get(position.index)?;
            let is_current = scene.graph.is_current(position.index);
            Some(node_widget(node, is_current).draw(node_area, buf))
        })
        .flatten()
        .collect();

    render_scrollbars(
        buf,
        area,
        scene.layout.content_width,
        scene.layout.content_height,
        scroll.x,
        scroll.y,
    );
    click_targets
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeShape {
    pub node_count: usize,
    pub current_node_id: Option<NodeId>,
}

fn tree_shape<D: Dag>(dag: &D) -> TreeShape {
    TreeShape {
        node_count: dag.node_count(),
        current_node_id: dag.current_node(),
    }
}

fn current_node_selection(node_id: NodeId, selectable_items: &[Selection]) -> Option<Selection> {
    let active_goal = Selection::Goal {
        node_id,
        goal_idx: 0,
    };
    if selectable_items.contains(&active_goal) {
        return Some(active_goal);
    }
    selectable_items
        .iter()
        .copied()
        .find(|item| item.node_id() == node_id)
}

pub type GraphSelection = AnchoredSelection<Selection, TreeShape>;

impl GraphSelection {
    pub fn current<D: Dag>(&self, dag: &D, selectable_items: &[Selection]) -> Option<Selection> {
        self.value_at(&tree_shape(dag))
            .filter(|selection| selectable_items.contains(selection))
            .or_else(|| current_node_selection(dag.current_node()?, selectable_items))
    }

    pub fn click<D: Dag>(&mut self, clicked: Selection, dag: &D, selectable_items: &[Selection]) {
        if selectable_items.contains(&clicked) {
            self.pin(clicked, tree_shape(dag));
        }
    }

    pub fn step<D>(
        &mut self,
        dag: &D,
        layout: &DagLayout,
        selectable_items: &[Selection],
        direction: Direction,
    ) where
        D: Dag,
        D::Node: NavNode,
    {
        let Some(current) = self.current(dag, selectable_items) else {
            return;
        };
        let target =
            find_nearest_in_direction(&build_navigation_regions(dag, layout), current, direction)
                .filter(|target| selectable_items.contains(target))
                .unwrap_or(current);
        self.pin(target, tree_shape(dag));
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TreeGraphView {
    pub selection: GraphSelection,
    pub top_down: bool,
}

impl Default for TreeGraphView {
    fn default() -> Self {
        Self {
            selection: GraphSelection::default(),
            top_down: true,
        }
    }
}

impl TreeGraphView {
    pub const fn toggle(&mut self, action: ModeAction) {
        if matches!(action, ModeAction::FlipOrientation) {
            self.top_down = !self.top_down;
        }
    }
}

fn center_on(target: &NodePosition, layout: &DagLayout, viewport: Rect) -> ScrollOffset {
    let center_x = target.x + i32::from(target.width) / 2;
    let center_y = target.y + i32::from(target.height) / 2;

    let max_x = (layout.content_width - i32::from(viewport.width)).max(0);
    let max_y = (layout.content_height - i32::from(viewport.height)).max(0);

    ScrollOffset {
        x: (center_x - i32::from(viewport.width) / 2).clamp(0, max_x),
        y: (center_y - i32::from(viewport.height) / 2).clamp(0, max_y),
    }
}

fn clip_to_viewport(position: &NodePosition, scroll: ScrollOffset, viewport: Rect) -> Option<Rect> {
    let relative_x = position.x - scroll.x;
    let relative_y = position.y - scroll.y;

    let outside = relative_x >= i32::from(viewport.width)
        || relative_y >= i32::from(viewport.height)
        || relative_x + i32::from(position.width) <= 0
        || relative_y + i32::from(position.height) <= 0;
    if outside {
        return None;
    }

    let horizontal = clip_axis(relative_x, position.width, viewport.x, viewport.width);
    let vertical = clip_axis(relative_y, position.height, viewport.y, viewport.height);

    (horizontal.length > 0 && vertical.length > 0).then_some(Rect::new(
        horizontal.start,
        vertical.start,
        horizontal.length,
        vertical.length,
    ))
}

struct ClippedSpan {
    start: u16,
    length: u16,
}

fn clip_axis(relative_position: i32, size: u16, origin: u16, area_size: u16) -> ClippedSpan {
    if relative_position < 0 {
        let clipped = (-relative_position) as u16;
        ClippedSpan {
            start: origin,
            length: size.saturating_sub(clipped).min(area_size),
        }
    } else {
        let offset = relative_position as u16;
        ClippedSpan {
            start: origin + offset,
            length: size.min(area_size.saturating_sub(offset)),
        }
    }
}
