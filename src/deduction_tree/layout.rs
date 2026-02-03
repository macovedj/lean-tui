//! Layout for the semantic tableau - combines given, proof, and theorem panes.

use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    widgets::StatefulWidget,
};

use super::{
    given_pane::{GivenPane, GivenPaneState},
    proof_pane::{ProofPane, ProofPaneState},
    theorem_pane::{TheoremPane, TheoremPaneState},
    types::CompleteProofDag,
};
use crate::{
    protocol::{NodeId, TacticProofState},
    shared::Selection,
};

/// Combined state for the semantic tableau layout.
#[derive(Default)]
pub struct SemanticTableauState {
    pub given: GivenPaneState,
    pub proof: ProofPaneState,
    pub theorem: TheoremPaneState,
}

impl SemanticTableauState {
    pub fn find_click_at(&self, x: u16, y: u16) -> Option<Selection> {
        self.given
            .find_click_at(x, y)
            .or_else(|| self.proof.find_click_at(x, y))
            .or_else(|| self.theorem.find_click_at(x, y))
    }

    pub fn update_current_node(&mut self, current_node: Option<NodeId>) {
        self.proof.update_current_node(current_node);
    }
}

/// Semantic tableau layout widget - combines given, proof, and theorem panes.
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
        let (given_area, proof_area, theorem_area) = compute_layout(area, self.top_down);

        let initial = self.dag.initial_proof_state();
        GivenPane::new(&initial.hypotheses, self.selection).render(
            given_area,
            buf,
            &mut state.given,
        );

        ProofPane::new(self.dag, self.top_down, self.selection, self.current_state).render(
            proof_area,
            buf,
            &mut state.proof,
        );

        let theorem_goal = initial
            .goals
            .first()
            .map(|g| g.type_.to_plain_text())
            .unwrap_or_default();
        TheoremPane::new(&theorem_goal, self.selection).render(
            theorem_area,
            buf,
            &mut state.theorem,
        );
    }
}

fn compute_layout(area: Rect, top_down: bool) -> (Rect, Rect, Rect) {
    if top_down {
        Layout::vertical([
            Constraint::Length(3),
            Constraint::Fill(1),
            Constraint::Length(3),
        ])
        .areas::<3>(area)
        .into()
    } else {
        let [given, theorem, proof] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Fill(1),
        ])
        .areas(area);
        (given, proof, theorem)
    }
}
