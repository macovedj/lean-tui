//! Effect flow display style - monadic effect decomposition visualization.
//!
//! This style visualizes effect flow using forward/backward function
//! decomposition, inspired by Aeneas.

mod mode;
pub mod types;

pub use mode::{EffectFlowMode, EffectFlowModeInput};
