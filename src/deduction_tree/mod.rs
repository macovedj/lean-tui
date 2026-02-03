//! Deduction tree display style - semantic tableau visualization of proofs.

mod given_pane;
pub mod layout;
mod mode;
mod proof_pane;
mod state_node;
mod theorem_pane;
pub mod types;

pub use layout::{SemanticTableauLayout, SemanticTableauState};
pub use mode::{DeductionTreeModeInput, SemanticTableau};
pub use types::{
    CompleteProofDag, NodeId, ProofDagConversionError, ProofDagTacticNode, TacticText,
};
