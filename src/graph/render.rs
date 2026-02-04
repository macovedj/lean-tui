use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget},
};

use super::scroll::{needs_horizontal_scroll, needs_vertical_scroll};

pub fn render_scrollbars(
    buf: &mut Buffer,
    area: Rect,
    content_width: i32,
    content_height: i32,
    scroll_x: i32,
    scroll_y: i32,
) {
    if needs_vertical_scroll(content_height, area) && area.height > 2 {
        let sb_area = Rect::new(
            area.x + area.width.saturating_sub(1),
            area.y,
            1,
            area.height,
        );
        let mut sb_state = ScrollbarState::new(content_height.max(0) as usize)
            .position(scroll_y.max(0) as usize)
            .viewport_content_length(area.height as usize);
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .render(sb_area, buf, &mut sb_state);
    }

    if needs_horizontal_scroll(content_width, area) && area.width > 2 {
        let sb_area = Rect::new(
            area.x,
            area.y + area.height.saturating_sub(1),
            area.width.saturating_sub(1),
            1,
        );
        let mut sb_state = ScrollbarState::new(content_width.max(0) as usize)
            .position(scroll_x.max(0) as usize)
            .viewport_content_length(area.width as usize);
        Scrollbar::new(ScrollbarOrientation::HorizontalBottom)
            .begin_symbol(Some("◀"))
            .end_symbol(Some("▶"))
            .render(sb_area, buf, &mut sb_state);
    }
}
