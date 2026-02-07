use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, StatefulWidget, Widget},
};

use crate::shared::{theme::Theme, FilterToggle, HypothesisFilters, InteractiveStatefulWidget};

/// Input for updating status bar state.
pub struct StatusBarInput {
    pub filters: HypothesisFilters,
    pub keybindings: &'static [(&'static str, &'static str)],
    pub supported_filters: &'static [FilterToggle],
}

/// State for the status bar widget.
#[derive(Default)]
pub struct StatusBar {
    filters: HypothesisFilters,
    keybindings: &'static [(&'static str, &'static str)],
    supported_filters: &'static [FilterToggle],
}

/// Widget for rendering the status bar.
pub struct StatusBarWidget;

impl StatefulWidget for StatusBarWidget {
    type State = StatusBar;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        const GLOBAL_KEYBINDINGS: &[(&str, &str)] = &[
            ("?", "help"),
            ("j/k", "nav"),
            ("[/]", "mode"),
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

        let mode_spans = state.keybindings.iter().flat_map(|(key, desc)| {
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

        let filter_status = build_filter_status(state.filters, state.supported_filters);
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

impl InteractiveStatefulWidget for StatusBarWidget {
    type Input = StatusBarInput;
    type Event = ();

    fn update_state(state: &mut Self::State, input: Self::Input) {
        state.filters = input.filters;
        state.keybindings = input.keybindings;
        state.supported_filters = input.supported_filters;
    }
}

fn build_filter_status(filters: HypothesisFilters, supported: &[FilterToggle]) -> String {
    [
        (FilterToggle::Instances, filters.hide_instances, 'i'),
        (FilterToggle::Inaccessible, filters.hide_inaccessible, 'a'),
        (FilterToggle::LetValues, filters.hide_let_values, 'l'),
        (FilterToggle::ReverseOrder, filters.reverse_order, 'r'),
    ]
    .into_iter()
    .filter(|(toggle, _, _)| supported.contains(toggle))
    .filter_map(|(_, enabled, c)| enabled.then_some(c))
    .collect()
}
