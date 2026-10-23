use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Clear, Paragraph, Widget},
};

use crate::{
    keybindings::{KeyBinding, GLOBAL_KEYBINDINGS},
    message::ModeAction,
    theme::Theme,
};

fn help_lines<A>(bindings: &[KeyBinding<A>]) -> impl Iterator<Item = Line<'static>> + '_ {
    let key_style = Style::new().fg(Theme::KEYBINDING);
    bindings.iter().map(move |binding| {
        Line::from(vec![
            Span::styled(format!("{:>6}", binding.label), key_style),
            Span::raw(format!("  {}", binding.help_description)),
        ])
    })
}

pub struct HelpMenu {
    pub mode_keybindings: &'static [KeyBinding<ModeAction>],
}

impl Widget for HelpMenu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines: Vec<Line> = help_lines(GLOBAL_KEYBINDINGS)
            .chain(help_lines(self.mode_keybindings))
            .collect();

        let width = lines.iter().map(Line::width).max().unwrap_or(0) as u16 + 2;
        let height = (lines.len() as u16) + 2;
        let x = area.width.saturating_sub(width + 1);
        let y = area.height.saturating_sub(height + 2);
        let popup_area = Rect::new(x, y, width, height).intersection(area);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title(" Help ")
            .border_style(Style::new().fg(Color::Reset));

        Paragraph::new(lines).block(block).render(popup_area, buf);
    }
}
