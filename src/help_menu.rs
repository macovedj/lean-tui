use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, StatefulWidget, Widget},
};

use crate::shared::{theme::Theme, InteractiveStatefulWidget, KeyEvent};

const KEYBINDINGS: &[(&str, &str)] = &[
    ("[/]", "cycle display mode"),
    ("j/k", "navigate up/down"),
    ("g", "goto origin"),
    ("y", "copy to clipboard"),
    ("f", "select file"),
    ("F", "toggle follow mode"),
    ("L", "legend"),
    ("?", "close help"),
    ("q", "quit"),
];

/// State for the help menu widget.
#[derive(Default)]
pub struct HelpMenu {
    visible: bool,
}

impl HelpMenu {
    pub const fn toggle(&mut self) {
        self.visible = !self.visible;
    }
}

/// Widget for rendering the help menu overlay.
pub struct HelpMenuWidget;

impl StatefulWidget for HelpMenuWidget {
    type State = HelpMenu;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

        let key_style = Style::new().fg(Theme::KEYBINDING);

        let lines: Vec<Line> = KEYBINDINGS
            .iter()
            .map(|(key, desc)| {
                Line::from(vec![
                    Span::styled(format!("{key:>6}"), key_style),
                    Span::raw(format!("  {desc}")),
                ])
            })
            .collect();

        let width = 28u16;
        let height = (lines.len() as u16) + 2;
        let x = area.width.saturating_sub(width + 1);
        let y = area.height.saturating_sub(height + 2);
        let popup_area = Rect::new(x, y, width, height);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title(" Help ")
            .border_style(Style::new().fg(Color::Reset));

        Paragraph::new(lines).block(block).render(popup_area, buf);
    }
}

impl InteractiveStatefulWidget for HelpMenuWidget {
    type Input = ();
    type Event = KeyEvent;

    fn update_state(_state: &mut Self::State, _input: Self::Input) {}

    fn handle_event(state: &mut Self::State, event: Self::Event) -> bool {
        if !state.visible {
            return false;
        }

        match event.code {
            KeyCode::Esc | KeyCode::Char('?') => {
                state.visible = false;
                true
            }
            _ => false,
        }
    }
}
