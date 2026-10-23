use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    prelude::Stylize,
    style::Color,
    widgets::{Paragraph, Widget},
};

use super::error_banner::ErrorBanner;

pub fn render_error(error: &str, area: Rect, buf: &mut Buffer) -> Rect {
    let banner = ErrorBanner::new(error);
    let height = banner.required_height(area.width);

    let [error_area, rest] =
        Layout::vertical([Constraint::Length(height), Constraint::Fill(1)]).areas(area);

    banner.render(error_area, buf);
    rest
}

pub fn render_no_goals(area: Rect, buf: &mut Buffer) {
    Paragraph::new("No goals")
        .fg(Color::DarkGray)
        .render(area, buf);
}
