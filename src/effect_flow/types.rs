//! Effect flow types.
//!
//! Helper types and extension traits for working with `GenericDag` in
//! effectflow display style.

use crate::protocol::{GenericDag, GraphNode};

/// Extension trait for `GenericDag` in effectflow mode.
pub trait EffectFlowDagExt {
    /// Get the monad stack from DAG metadata.
    fn monad_stack(&self) -> Option<Vec<String>>;
}

impl EffectFlowDagExt for GenericDag {
    fn monad_stack(&self) -> Option<Vec<String>> {
        self.metadata
            .get("monad_stack")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }
}

/// Extension trait for `GraphNode` in effectflow mode.
pub trait EffectFlowNodeExt {
    /// Get forward interpretation from node metadata.
    fn forward(&self) -> Option<String>;

    /// Get backward interpretation from node metadata.
    fn backward(&self) -> Option<String>;

    /// Get bindings read by this effect.
    fn bindings_read(&self) -> Vec<String>;

    /// Get bindings written by this effect.
    fn bindings_written(&self) -> Vec<String>;
}

impl EffectFlowNodeExt for GraphNode {
    fn forward(&self) -> Option<String> {
        self.metadata
            .get("forward")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    fn backward(&self) -> Option<String> {
        self.metadata
            .get("backward")
            .and_then(|v| v.as_str())
            .map(String::from)
    }

    fn bindings_read(&self) -> Vec<String> {
        self.metadata
            .get("bindings_read")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }

    fn bindings_written(&self) -> Vec<String> {
        self.metadata
            .get("bindings_written")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default()
    }
}
