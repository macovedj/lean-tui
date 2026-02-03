//! Tactic tree display style - sidebar proof steps with hypotheses and goals.

mod mode;
pub mod proof_steps_sidebar;

pub use mode::{StepsModeInput, TacticTree};
pub use proof_steps_sidebar::{ProofStepsSidebar, ProofStepsSidebarState};
