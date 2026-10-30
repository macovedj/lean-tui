use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Clear, Widget},
    Frame,
};

use crate::{
    message::ClickTarget,
    model::{ActiveWorker, Model, Overlay, WorkerConnection},
    theme::Theme,
    widgets::{
        connection_hint::ConnectionHint, error_banner::ErrorBanner, help_menu::HelpMenu,
        input_pane::InputPane, legend::LegendPopup, output_pane::OutputPane,
        render_helpers::render_error, status_bar::StatusBar, worker_selector::WorkerSelector,
    },
};

fn file_name_of(uri: &str) -> &str {
    uri.rsplit('/').next().unwrap_or(uri)
}

pub fn view(model: &Model, frame: &mut Frame) -> Vec<ClickTarget> {
    let area = frame.area();
    let buf = frame.buffer_mut();
    let [main_area, status_area] =
        Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(area);

    let click_targets = draw_main(model, main_area, buf);
    StatusBar {
        mode_keybindings: model.display_mode.keybindings(),
    }
    .render(status_area, buf);
    draw_overlay(model, area, main_area, buf);
    click_targets
}

fn draw_overlay(model: &Model, area: Rect, main_area: Rect, buf: &mut Buffer) {
    match &model.overlay {
        Some(Overlay::Help) => HelpMenu {
            mode_keybindings: model.display_mode.keybindings(),
        }
        .render(area, buf),
        Some(Overlay::Legend) => LegendPopup {
            sections: model.display_mode.legend_sections(model.dag()),
        }
        .render(area, buf),
        Some(Overlay::WorkerSelector {
            highlighted,
            rescanning,
        }) => WorkerSelector {
            workers: &model.known_workers,
            highlighted: *highlighted,
            rescanning: *rescanning,
            port_range: model.port_range.clone(),
        }
        .render(area, buf),
        Some(Overlay::Error { message }) => {
            let banner = ErrorBanner::new(message);
            let height = banner
                .required_height(main_area.width)
                .min(main_area.height / 2);
            let overlay_area = Rect::new(main_area.x, main_area.y, main_area.width, height);
            Clear.render(overlay_area, buf);
            banner.render(overlay_area, buf);
        }
        None => {}
    }
}

fn connection_hint(model: &Model) -> Option<ConnectionHint<'_>> {
    let port_min = *model.port_range.start();
    let port_max = *model.port_range.end();
    match &model.connection {
        WorkerConnection::Searching => {
            Some(ConnectionHint::SearchingWorkers { port_min, port_max })
        }
        WorkerConnection::Stopped { file_uri, .. } => Some(ConnectionHint::WorkerStopped {
            file_name: file_name_of(file_uri),
            port_min,
            port_max,
        }),
        WorkerConnection::Active(worker) if worker.dag.is_some() || worker.error.is_some() => None,
        WorkerConnection::Active(worker) => match &worker.cursor {
            None => Some(ConnectionHint::AwaitingCursor {
                file_name: worker.file_uri.as_deref().map(file_name_of),
                editor_name: worker.editor_name.as_deref(),
                other_worker_count: model.known_workers.len().saturating_sub(1),
            }),
            Some(cursor) if worker.awaiting_graph => Some(ConnectionHint::AwaitingGraph {
                file_name: file_name_of(&cursor.uri),
                line_number: cursor.position.line + 1,
            }),
            Some(_) => None,
        },
    }
}

fn block_title(worker: &ActiveWorker) -> String {
    match (&worker.definition, &worker.cursor) {
        (Some(definition), Some(cursor)) => {
            let filename = cursor.filename().unwrap_or("?");
            definition.kind.as_ref().map_or_else(
                || format!(" {} ({}) ", definition.name, filename),
                |kind| format!(" {kind} {} ({}) ", definition.name, filename),
            )
        }
        (None, Some(cursor)) => format!("lean-tui ({}) ", cursor.filename().unwrap_or("?")),
        (_, None) => "lean-tui".to_string(),
    }
}

fn position_info(follow_mode: bool, worker: &ActiveWorker) -> String {
    let follow = if follow_mode { "follow" } else { "manual" };
    worker.cursor.as_ref().map_or_else(
        || format!(" [{follow}] "),
        |cursor| {
            format!(
                " {}:{} ({}) [{}] ",
                cursor.position.line + 1,
                cursor.position.character + 1,
                cursor.method,
                follow
            )
        },
    )
}

fn draw_main(model: &Model, area: Rect, buf: &mut Buffer) -> Vec<ClickTarget> {
    match (connection_hint(model), &model.connection) {
        (Some(hint), _) => {
            hint.render(area, buf);
            Vec::new()
        }
        (None, WorkerConnection::Active(worker)) => draw_active_worker(model, worker, area, buf),
        (None, _) => Vec::new(),
    }
}

fn draw_active_worker(
    model: &Model,
    worker: &ActiveWorker,
    area: Rect,
    buf: &mut Buffer,
) -> Vec<ClickTarget> {
    let block = Block::bordered()
        .title(block_title(worker))
        .title_top(Line::from(format!(" {} ", model.display_mode.name())).right_aligned())
        .title_bottom(Line::from(position_info(model.follow_mode, worker)).right_aligned())
        .border_style(Style::new().fg(Theme::FRAME_BORDER));

    let inner = block.inner(area);
    block.render(area, buf);

    let dag = &worker.dag;
    let max_pane_height = inner.height / 4;
    let input_height = if model.show_input {
        (InputPane::required_height(dag.block_input(), inner.width) + 2).min(max_pane_height)
    } else {
        0
    };
    let output_height = if model.show_output {
        (OutputPane::required_height(dag.block_output(), inner.width) + 2).min(max_pane_height)
    } else {
        0
    };
    let [input_area, mode_area, output_area] = Layout::vertical([
        Constraint::Length(input_height),
        Constraint::Fill(1),
        Constraint::Length(output_height),
    ])
    .areas(inner);

    if model.show_input {
        InputPane {
            bindings: dag.block_input(),
        }
        .render(input_area, buf);
    }
    let error = worker
        .error
        .clone()
        .or_else(|| model.display_mode.incompatibility_error(dag));
    let mode_area = error.map_or(mode_area, |error| render_error(&error, mode_area, buf));
    let click_targets = model
        .display_mode
        .draw(dag, model.timing_display, mode_area, buf);
    if model.show_output {
        OutputPane {
            block_output: dag.block_output(),
        }
        .render(output_area, buf);
    }
    click_targets
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;

    use crate::scenario_replay::{key, render_scenario, ReplayedMessages};

    #[test]
    fn searching_without_messages() {
        assert_snapshot!(render_scenario(
            "swap",
            &ReplayedMessages::First { count: 0 },
            &[]
        ));
    }

    #[test]
    fn awaiting_graph_after_cursor() {
        assert_snapshot!(render_scenario(
            "swap",
            &ReplayedMessages::First { count: 3 },
            &[]
        ));
    }

    #[test]
    fn swap_input_output_panes() {
        assert_snapshot!(render_scenario(
            "swap",
            &ReplayedMessages::All,
            &[key('i'), key('o')]
        ));
    }

    #[test]
    fn swap_help_menu() {
        assert_snapshot!(render_scenario("swap", &ReplayedMessages::All, &[key('?')]));
    }

    #[test]
    fn swap_worker_selector() {
        assert_snapshot!(render_scenario("swap", &ReplayedMessages::All, &[key('f')]));
    }
}
