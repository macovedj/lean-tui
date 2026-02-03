use ratatui::{
    layout::{Constraint, Layout, Rect},
    prelude::Stylize,
    style::Color,
    widgets::Paragraph,
    Frame,
};

use super::error_banner::ErrorBanner;

/// Render an error message at the top of the area and return the remaining
/// area. Returns the original area if there's no error.
pub fn render_error(frame: &mut Frame, area: Rect, error: Option<&str>) -> Rect {
    let Some(error) = error else {
        return area;
    };

    let banner = ErrorBanner::new(error);
    let height = banner.required_height(area.width);

    let [error_area, rest] =
        Layout::vertical([Constraint::Length(height), Constraint::Fill(1)]).areas(area);

    frame.render_widget(banner, error_area);
    rest
}

/// Render a "No goals" placeholder when goals are empty.
pub fn render_no_goals(frame: &mut Frame, area: Rect) {
    frame.render_widget(Paragraph::new("No goals").fg(Color::DarkGray), area);
}
