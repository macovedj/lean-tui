use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::theme::Theme;

/// Stateless status bar widget.
pub struct StatusBar<'a> {
    keybindings: &'a [(&'static str, &'static str)],
}

impl<'a> StatusBar<'a> {
    pub fn new(keybindings: &'a [(&'static str, &'static str)]) -> Self {
        Self { keybindings }
    }
}

impl Widget for StatusBar<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        const GLOBAL_KEYBINDINGS: &[(&str, &str)] = &[
            ("?", "help"),
            ("[/]", "style"),
            ("i/o", "in/out"),
            ("m", "metrics"),
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

        let spans: Vec<Span> = global_spans.chain(mode_spans).chain(nav_spans).collect();
        Paragraph::new(Line::from(spans)).render(area, buf);
    }
}
