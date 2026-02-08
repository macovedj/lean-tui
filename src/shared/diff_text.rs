use ratatui::{
    style::{Modifier, Style},
    text::Span,
};

use super::theme::Theme;
use crate::protocol::{DiffStatus, RichText};

pub const fn diff_status_style(status: DiffStatus, base_style: Style) -> Style {
    match status {
        DiffStatus::Changed => base_style.fg(Theme::CHANGED),
        DiffStatus::Added => base_style.fg(Theme::ADDED),
        DiffStatus::Removed => base_style.fg(Theme::REMOVED).add_modifier(Modifier::DIM),
    }
}

pub trait RichTextExt {
    fn to_spans(&self, base_style: Style) -> Vec<Span<'static>>;
}

impl RichTextExt for RichText {
    fn to_spans(&self, base_style: Style) -> Vec<Span<'static>> {
        match self {
            Self::Text { text } => vec![Span::styled(text.clone(), base_style)],
            Self::Diff { diff, content } => {
                let style = diff_status_style(*diff, base_style);
                content.to_spans(style)
            }
            Self::Append { children } => children
                .iter()
                .flat_map(|item| item.to_spans(base_style))
                .collect(),
        }
    }
}
