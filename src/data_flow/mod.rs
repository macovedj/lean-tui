//! Data flow display style - functional term decomposition visualization.

mod given_pane;
pub mod layout;
mod mode;
mod state_node;
pub mod types;

pub use layout::{FunctionalTableauLayout, FunctionalTableauState};
pub use mode::{DataFlowMode, DataFlowModeInput};
pub use types::{DataFlowDagExt, DataFlowNodeExt, NodeId};
