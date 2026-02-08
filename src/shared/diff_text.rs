use ratatui::{
    style::{Color, Modifier, Style},
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

pub const fn item_style(is_selected: bool, fg_color: Color) -> Style {
    if is_selected {
        Style::new().fg(fg_color).add_modifier(Modifier::UNDERLINED)
    } else {
        Style::new().fg(fg_color)
    }
}

pub struct DiffState {
    pub is_inserted: bool,
    pub is_removed: bool,
}

pub struct DiffStyle {
    pub style: Style,
}

pub const fn diff_style(state: &DiffState, is_selected: bool, base_color: Color) -> DiffStyle {
    if state.is_inserted {
        DiffStyle {
            style: item_style(is_selected, Theme::ADDED),
        }
    } else if state.is_removed {
        DiffStyle {
            style: item_style(is_selected, Theme::REMOVED).add_modifier(Modifier::CROSSED_OUT),
        }
    } else {
        DiffStyle {
            style: item_style(is_selected, base_color),
        }
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
