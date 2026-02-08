use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::shared::{theme::Theme, FilterToggle};

/// Stateless status bar widget.
pub struct StatusBar<'a> {
    keybindings: &'a [(&'static str, &'static str)],
    toggles: &'a [FilterToggle],
    filters: &'a [FilterToggle],
}

impl<'a> StatusBar<'a> {
    pub fn new(
        keybindings: &'a [(&'static str, &'static str)],
        toggles: &'a [FilterToggle],
        filters: &'a [FilterToggle],
    ) -> Self {
        Self {
            keybindings,
            toggles,
            filters,
        }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const GLOBAL_KEYBINDINGS: &[(&str, &str)] = &[
            ("?", "help"),
            ("j/k", "nav"),
            ("[/]", "mode"),
            ("i/o", "in/out"),
            ("q", "quit"),
        ];

        let separator = Span::raw(" │ ");

        let key_style = Style::new().fg(Theme::KEYBINDING);

        let global_spans = GLOBAL_KEYBINDINGS
            .iter()
            .enumerate()
            .flat_map(|(i, (key, desc))| {
                let prefix = (i > 0).then(|| separator.clone());
                prefix.into_iter().chain([
                    Span::styled(*key, key_style),
                    Span::raw(format!(": {desc}")),
                ])
            });

        let mode_spans = self.keybindings.iter().flat_map(|(key, desc)| {
            [
                separator.clone(),
                Span::styled(*key, key_style),
                Span::raw(format!(": {desc}")),
            ]
        });

        let nav_spans = [
            separator.clone(),
            Span::styled("g", key_style),
            Span::raw(": goto"),
            Span::raw(" "),
            Span::styled("y", key_style),
            Span::raw(": copy"),
        ];

        let filter_status = build_filter_status(self.toggles, self.filters);
        let filter_span = (!filter_status.is_empty())
            .then(|| Span::styled(format!(" [{filter_status}]"), Style::new().fg(Theme::ADDED)));

        let spans: Vec<Span> = global_spans
            .chain(mode_spans)
            .chain(nav_spans)
            .chain(filter_span)
            .collect();
        Paragraph::new(Line::from(spans)).render(area, buf);
    }
}

fn build_filter_status(toggles: &[FilterToggle], filters: &[FilterToggle]) -> String {
    toggles
        .iter()
        .filter(|toggle| filters.contains(toggle))
        .map(|toggle| toggle.indicator())
        .collect()
}
