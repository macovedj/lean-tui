//! Layout for the semantic tableau - the proof pane only.
//! Input and output panes are now global and rendered at the app level.

use ratatui::{buffer::Buffer, layout::Rect, widgets::StatefulWidget};

use super::{
    proof_pane::{ProofPane, ProofPaneState},
    types::CompleteProofDag,
};
use crate::{
    deduction_tree::TacticProofState,
    protocol::NodeId,
    selection::{find_region_at, Region, Selection},
};

/// Combined state for the semantic tableau layout.
#[derive(Default)]
pub struct SemanticTableauState {
    pub proof: ProofPaneState,
}

impl SemanticTableauState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        find_region_at(self.proof.click_regions.as_slice(), x, y)
    }

    pub fn navigation_regions(&self) -> Vec<Region> {
        self.proof.navigation_regions().to_vec()
    }

    pub fn update_current_node(&mut self, current_node: Option<NodeId>) {
        self.proof.update_current_node(current_node);
    }
}

/// Semantic tableau layout widget - renders the proof pane.
pub struct SemanticTableauLayout<'a> {
    dag: &'a CompleteProofDag,
    top_down: bool,
    selection: Option<Selection>,
    current_state: &'a TacticProofState,
}

impl<'a> SemanticTableauLayout<'a> {
    pub const fn new(
        dag: &'a CompleteProofDag,
        top_down: bool,
        selection: Option<Selection>,
        current_state: &'a TacticProofState,
    ) -> Self {
        Self {
            dag,
            top_down,
            selection,
            current_state,
        }
    }
}

impl StatefulWidget for SemanticTableauLayout<'_> {
    type State = SemanticTableauState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        ProofPane::new(self.dag, self.top_down, self.selection, self.current_state).render(
            area,
            buf,
            &mut state.proof,
        );
    }
}
