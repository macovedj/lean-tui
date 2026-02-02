//! Tableau widgets for proof and functional DAG visualization.
//!
//! This module contains shared components and specialized widgets for:
//! - Semantic tableau (tactic proof visualization)
//! - Functional tableau (term-mode dataflow visualization)

mod canvas;
mod navigation;
mod tree_layout;

pub mod functional;
pub mod semantic;

// Re-export shared types
pub use canvas::VirtualCanvas;
// Re-export main layout widgets
pub use functional::{FunctionalTableauLayout, FunctionalTableauState};
pub use navigation::{find_nearest_in_direction, Direction, NavigationRegion};
pub use semantic::{SemanticTableauLayout, SemanticTableauState};
pub use tree_layout::{NodePosition, TreeLayout, ORPHAN_GAP};

/// Truncate a string to max length with ellipsis.
pub fn truncate_str(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_string()
    } else {
        chars
            .into_iter()
            .take(max.saturating_sub(3))
            .chain(['.', '.', '.'])
            .collect()
    }
}
