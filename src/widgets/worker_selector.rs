use std::mem;

use crossterm::event::KeyCode;
use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Clear, List, ListItem, ListState, Paragraph, StatefulWidget, Widget},
};

use crate::{
    interactive::{InteractiveStatefulWidget, KeyEvent},
    tcp_client::WorkerInfo,
    theme::Theme,
};

/// Result of user interaction with the worker selector.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerSelectorResult {
    /// No action taken yet.
    Pending,
    /// User selected a specific worker.
    Selected(u16),
    /// User wants to scan for workers.
    Rescan,
    /// User cancelled the dialog.
    Cancelled,
}

/// State for the worker selector dialog.
pub struct WorkerSelectorState {
    /// Whether the dialog is visible.
    pub visible: bool,
    /// Discovered workers.
    pub workers: Vec<WorkerInfo>,
    /// Currently selected index in the list.
    list_state: ListState,
    /// Whether we're currently scanning.
    pub scanning: bool,
    /// Port range for scanning.
    pub port_range: (u16, u16),
    /// Result of the last interaction.
    pub result: WorkerSelectorResult,
}

impl Default for WorkerSelectorState {
    fn default() -> Self {
        Self {
            visible: false,
            workers: Vec::new(),
            list_state: ListState::default(),
            scanning: false,
            port_range: (9742, 9752),
            result: WorkerSelectorResult::Pending,
        }
    }
}

impl WorkerSelectorState {
    pub fn show(&mut self) {
        self.visible = true;
        self.result = WorkerSelectorResult::Pending;
        if !self.workers.is_empty() && self.list_state.selected().is_none() {
            self.list_state.select(Some(0));
        }
    }

    pub fn set_workers(&mut self, workers: Vec<WorkerInfo>) {
        self.workers = workers;
        self.scanning = false;
        if self.workers.is_empty() {
            self.list_state.select(None);
        } else {
            self.list_state.select(Some(0));
        }
    }

    pub fn set_port_range(&mut self, min: u16, max: u16) {
        self.port_range = (min, max);
    }

    fn select_previous(&mut self) {
        if self.workers.is_empty() {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| i.saturating_sub(1));
        self.list_state.select(Some(i));
    }

    fn select_next(&mut self) {
        if self.workers.is_empty() {
            return;
        }
        let i = self
            .list_state
            .selected()
            .map_or(0, |i| (i + 1).min(self.workers.len() - 1));
        self.list_state.select(Some(i));
    }

    fn confirm_selection(&mut self) {
        if let Some(i) = self.list_state.selected() {
            if let Some(worker) = self.workers.get(i) {
                self.result = WorkerSelectorResult::Selected(worker.port);
                self.visible = false;
            }
        }
    }

    /// Take the result, resetting to Pending.
    pub fn take_result(&mut self) -> WorkerSelectorResult {
        mem::replace(&mut self.result, WorkerSelectorResult::Pending)
    }
}

/// Widget for rendering the worker selector dialog.
pub struct WorkerSelectorWidget;

impl StatefulWidget for WorkerSelectorWidget {
    type State = WorkerSelectorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if !state.visible {
            return;
        }

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

        let info_text = if state.scanning {
            format!(
                "Scanning ports {}–{}...",
                state.port_range.0, state.port_range.1
            )
        } else if state.workers.is_empty() {
            "No workers found. Press 'r' to rescan.".to_string()
        } else {
            format!("Found {} worker(s):", state.workers.len())
        };
        Paragraph::new(info_text)
            .style(Style::new().fg(Color::DarkGray))
            .render(info_area, buf);

        if !state.workers.is_empty() {
            let items: Vec<ListItem> = state
                .workers
                .iter()
                .map(|w| Self::worker_to_list_item(w))
                .collect();

            let list = List::new(items)
                .highlight_style(
                    Style::new()
                        .fg(Color::White)
                        .bg(Theme::SELECTION_BG)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▸ ");

            StatefulWidget::render(list, list_area, buf, &mut state.list_state);
        }

        let key_style = Style::new().fg(Theme::KEYBINDING);
        let help_text = Line::from(vec![
            Span::styled("↑/↓", key_style),
            Span::raw(" select  "),
            Span::styled("Enter", key_style),
            Span::raw(" connect  "),
            Span::styled("r", key_style),
            Span::raw(" rescan  "),
            Span::styled("Esc", key_style),
            Span::raw(" cancel"),
        ]);
        Paragraph::new(help_text)
            .alignment(Alignment::Center)
            .render(help_area, buf);
    }
}

impl WorkerSelectorWidget {
    fn worker_to_list_item(w: &WorkerInfo) -> ListItem<'static> {
        let mut spans = vec![Span::raw(w.display_label())];
        if let Some(mode) = &w.server_mode {
            spans.push(Span::styled(
                format!(" [{mode}]"),
                Style::new().fg(Color::DarkGray),
            ));
        }
        ListItem::new(Line::from(spans))
    }
}

impl InteractiveStatefulWidget for WorkerSelectorWidget {
    type Input = ();
    type Event = KeyEvent;

    fn handle_event(state: &mut Self::State, event: Self::Event) -> bool {
        if !state.visible {
            return false;
        }

        match event.code {
            KeyCode::Up | KeyCode::Char('k') => {
                state.select_previous();
                true
            }
            KeyCode::Down | KeyCode::Char('j') => {
                state.select_next();
                true
            }
            KeyCode::Enter => {
                state.confirm_selection();
                true
            }
            KeyCode::Char('r') => {
                state.result = WorkerSelectorResult::Rescan;
                state.scanning = true;
                true
            }
            KeyCode::Esc => {
                state.result = WorkerSelectorResult::Cancelled;
                state.visible = false;
                true
            }
            _ => false,
        }
    }

    fn update_state(_state: &mut Self::State, _input: Self::Input) {}
}
