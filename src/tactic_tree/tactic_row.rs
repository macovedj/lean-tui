use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    widgets::Widget,
};

/// A horizontal divider.
pub struct Divider {
    style: Style,
}

impl Divider {
    pub const fn new() -> Self {
        Self {
            style: Style::new(),
        }
    }

    pub const fn style(mut self, style: Style) -> Self {
        self.style = style;
        self
    }
}

impl Default for Divider {
    fn default() -> Self {
        Self::new().style(Style::new().fg(Color::DarkGray))
    }
}

impl Widget for Divider {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let half = area.width.saturating_sub(1) / 2;
        let remainder = area.width.saturating_sub(1) % 2;
        let line = format!(
            "{}▼{}",
            "─".repeat(half as usize),
            "─".repeat((half + remainder) as usize)
        );

        buf.set_string(area.x, area.y, &line, self.style);
    }
}

/// Convenience function for simple divider rendering.
pub fn divider() -> Divider {
    Divider::default()
}
