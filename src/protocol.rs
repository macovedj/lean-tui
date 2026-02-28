// Include generated types (suppress lints for generated code)
#[allow(
    clippy::absolute_paths,
    clippy::clone_on_copy,
    clippy::default_trait_access,
    clippy::derivable_impls,
    clippy::derive_partial_eq_without_eq,
    clippy::doc_markdown,
    clippy::use_self,
    dead_code
)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/protocol_types.rs"));
}
pub use generated::*;

/// Type alias for node IDs.
pub type NodeId = u64;

impl ServerToClientMessage {
    /// Get a string describing the message type for logging.
    pub const fn message_type(&self) -> &'static str {
        match self {
            Self::Connected { .. } => "Connected",
            Self::Cursor { .. } => "Cursor",
            Self::Dag { .. } => "Dag",
            Self::Error { .. } => "Error",
            Self::FileFocused { .. } => "FileFocused",
        }
    }
}

impl Default for RichText {
    fn default() -> Self {
        Self::Text {
            text: String::new(),
        }
    }
}

impl RichText {
    /// Extract plain text from the tagged structure (flattens all diff
    /// markers).
    pub fn to_plain_text(&self) -> String {
        match self {
            Self::Text { text } => text.clone(),
            Self::Diff { content, .. } => content.to_plain_text(),
            Self::Append { children } => children.iter().map(Self::to_plain_text).collect(),
        }
    }
}

impl EditorCursorPosition {
    /// Extract filename from URI.
    pub fn filename(&self) -> Option<&str> {
        self.uri.rsplit('/').next()
    }
}

#[allow(clippy::derivable_impls)]
impl Default for LineCharacterPosition {
    fn default() -> Self {
        Self {
            line: 0,
            character: 0,
        }
    }
}

impl Copy for LineCharacterPosition {}

/// Check if a source range contains the cursor position.
/// Uses range containment: start <= cursor < end.
pub fn range_contains_cursor(
    start: LineCharacterPosition,
    end: Option<LineCharacterPosition>,
    cursor: LineCharacterPosition,
) -> bool {
    end.map_or(
        start.line == cursor.line && start.character <= cursor.character,
        |end| {
            (start.line < cursor.line
                || (start.line == cursor.line && start.character <= cursor.character))
                && (cursor.line < end.line
                    || (cursor.line == end.line && cursor.character < end.character))
        },
    )
}

/// Compute rough size of a source range for preferring more specific nodes.
pub fn range_size(start: LineCharacterPosition, end: Option<LineCharacterPosition>) -> u64 {
    end.map_or(0, |end| {
        (end.line.saturating_sub(start.line)) * 10000
            + end.character.saturating_sub(start.character)
    })
}

impl GraphNode {
    /// Check if this node's source range contains the cursor position.
    pub fn contains_cursor(&self, cursor: LineCharacterPosition) -> bool {
        range_contains_cursor(self.position, self.end_position, cursor)
    }

    /// Compute rough size of this node's range.
    pub fn range_size(&self) -> u64 {
        range_size(self.position, self.end_position)
    }

    pub fn elapsed_ns(&self) -> u64 {
        self.metadata
            .get("elapsed_ns")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
    }
}

pub fn format_elapsed_ns(ns: u64) -> String {
    if ns == 0 {
        return String::new();
    }
    let us = ns / 1_000;
    if us < 1_000 {
        format!("{us}µs")
    } else if us < 1_000_000 {
        let ms = us as f64 / 1_000.0;
        if ms < 10.0 {
            format!("{ms:.1}ms")
        } else {
            format!("{}ms", us / 1_000)
        }
    } else {
        let s = us as f64 / 1_000_000.0;
        format!("{s:.2}s")
    }
}
