use std::{iter::once, ops::RangeInclusive};

use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

use crate::{keybindings::WORKER_SELECTOR_KEYBINDINGS, tcp_client::WorkerInfo, theme::Theme};

pub struct WorkerSelector<'a> {
    pub workers: &'a [WorkerInfo],
    pub highlighted: Option<usize>,
    pub rescanning: bool,
    pub port_range: RangeInclusive<u16>,
}

fn worker_list_item(worker: &WorkerInfo) -> ListItem<'static> {
    let mode = worker
        .server_mode
        .as_ref()
        .map(|mode| Span::styled(format!(" [{mode}]"), Style::new().fg(Color::DarkGray)));
    ListItem::new(Line::from(
        once(Span::raw(worker.display_label()))
            .chain(mode)
            .collect::<Vec<_>>(),
    ))
}

fn selector_help_line() -> Line<'static> {
    let key_style = Style::new().fg(Theme::KEYBINDING);
    let last_index = WORKER_SELECTOR_KEYBINDINGS.len().saturating_sub(1);
    Line::from(
        WORKER_SELECTOR_KEYBINDINGS
            .iter()
            .enumerate()
            .flat_map(|(index, binding)| {
                let gap = if index == last_index { "" } else { "  " };
                [
                    Span::styled(binding.label, key_style),
                    Span::raw(format!(" {}{gap}", binding.help_description)),
                ]
            })
            .collect::<Vec<_>>(),
    )
}

impl Widget for WorkerSelector<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let width = 50u16.min(area.width.saturating_sub(4));
        let height = 14u16.min(area.height.saturating_sub(4));
        let x = (area.width.saturating_sub(width)) / 2;
        let y = (area.height.saturating_sub(height)) / 2;
        let popup_area = Rect::new(x, y, width, height);

        Clear.render(popup_area, buf);

        let block = Block::bordered()
            .title(" Select File ")
            .title_alignment(Alignment::Center)
            .border_style(Style::new().fg(Color::Reset));

        let inner = block.inner(popup_area);
        block.render(popup_area, buf);

        let [info_area, list_area, help_area] = Layout::vertical([
            Constraint::Length(2),
            Constraint::Fill(1),
            Constraint::Length(2),
        ])
        .areas(inner);

        let info_text = if self.rescanning {
            format!(
                "Scanning ports {}–{}...",
                self.port_range.start(),
                self.port_range.end()
            )
        } else if self.workers.is_empty() {
            "No workers found. Press 'r' to rescan.".to_string()
        } else {
            format!("Found {} worker(s):", self.workers.len())
        };
        Paragraph::new(info_text)
            .style(Style::new().fg(Color::DarkGray))
            .render(info_area, buf);

        if !self.workers.is_empty() {
            let list = List::new(self.workers.iter().map(worker_list_item))
                .highlight_style(
                    Style::new()
                        .fg(Color::White)
                        .bg(Theme::SELECTION_BG)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▸ ");

            StatefulWidget::render(
                list,
                list_area,
                buf,
                &mut ListState::default().with_selected(self.highlighted),
            );
        }

        Paragraph::new(selector_help_line())
            .alignment(Alignment::Center)
            .render(help_area, buf);
    }
}
