use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

use crate::{
    keybindings::{KeyBinding, GLOBAL_KEYBINDINGS},
    message::ModeAction,
    theme::Theme,
};

fn status_entries<A>(bindings: &[KeyBinding<A>]) -> impl Iterator<Item = [Span<'static>; 2]> + '_ {
    let key_style = Style::new().fg(Theme::KEYBINDING);
    bindings.iter().filter_map(move |binding| {
        binding.status_description.map(|description| {
            [
                Span::styled(binding.label, key_style),
                Span::raw(format!(": {description}")),
            ]
        })
    })
}

pub struct StatusBar {
    pub mode_keybindings: &'static [KeyBinding<ModeAction>],
}

impl Widget for StatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let spans: Vec<Span> = status_entries(GLOBAL_KEYBINDINGS)
            .chain(status_entries(self.mode_keybindings))
            .enumerate()
            .flat_map(|(index, entry)| {
                let separator = (index > 0).then(|| Span::raw(" │ "));
                separator.into_iter().chain(entry)
            })
            .collect();
        Paragraph::new(Line::from(spans)).render(area, buf);
    }
}
