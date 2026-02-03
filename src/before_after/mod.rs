//! Before/after display style - three-column temporal comparison of proof
//! states.

pub mod goals_column;
mod mode;

pub use goals_column::{GoalsColumn, GoalsColumnState};
pub use mode::{BeforeAfterMode, BeforeAfterModeInput};
