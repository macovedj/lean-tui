use crate::protocol::NodeId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    Hyp { node_id: NodeId, hyp_idx: usize },
    Goal { node_id: NodeId, goal_idx: usize },
    Error { node_id: NodeId, error_idx: usize },
}

impl Selection {
    pub const fn node_id(&self) -> NodeId {
        match *self {
            Self::Hyp { node_id, .. }
            | Self::Goal { node_id, .. }
            | Self::Error { node_id, .. } => node_id,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub x: i32,
    pub y: i32,
    pub width: u16,
    pub height: u16,
    pub selection: Selection,
}

#[derive(Debug, Clone, Copy)]
struct Pin<V, A> {
    value: V,
    anchor: A,
}

#[derive(Debug, Clone, Copy)]
pub struct AnchoredSelection<V, A> {
    pin: Option<Pin<V, A>>,
}

impl<V, A> Default for AnchoredSelection<V, A> {
    fn default() -> Self {
        Self { pin: None }
    }
}

impl<V: Copy, A: PartialEq> AnchoredSelection<V, A> {
    pub fn value_at(&self, anchor: &A) -> Option<V> {
        self.pin
            .as_ref()
            .filter(|pin| pin.anchor == *anchor)
            .map(|pin| pin.value)
    }

    pub fn pin(&mut self, value: V, anchor: A) {
        self.pin = Some(Pin { value, anchor });
    }
}

pub fn next_list_index(index: usize, item_count: usize) -> usize {
    if index + 1 < item_count {
        index + 1
    } else {
        index
    }
}

pub const fn previous_list_index(index: usize, _item_count: usize) -> usize {
    index.saturating_sub(1)
}
