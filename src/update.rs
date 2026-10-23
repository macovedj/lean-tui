use std::{io::stdout, mem};

use crossterm::{clipboard::CopyToClipboard, event::KeyCode, ExecutableCommand};
use futures::{future::BoxFuture, FutureExt};

use crate::{
    display_mode::DagState,
    keybindings::{find_key_action, GLOBAL_KEYBINDINGS, WORKER_SELECTOR_KEYBINDINGS},
    message::{AppMessage, GlobalAction, WorkerSelectorAction},
    model::{ActiveWorker, DefinitionInfo, Model, Overlay, WorkerConnection},
    protocol::{
        ClientToServerCommand, EditorCursorPosition, FullyQualifiedDocumentPosition,
        ServerToClientMessage,
    },
    selection::Selection,
    tcp_client::{discover_workers, WorkerInfo},
};

pub type Task = BoxFuture<'static, Option<AppMessage>>;

pub fn update(model: &mut Model, message: AppMessage) -> Option<Task> {
    match message {
        AppMessage::KeyPressed { key } => return apply_key(model, key.code),
        AppMessage::WorkerConnected { port, commands } => {
            model.worker_senders.insert(port, commands);
            let reconnects = match &model.connection {
                WorkerConnection::Searching => true,
                WorkerConnection::Stopped {
                    port: stopped_port, ..
                } => *stopped_port == port,
                WorkerConnection::Active(_) => false,
            };
            if reconnects {
                activate_worker(model, port);
            }
        }
        AppMessage::WorkerSent { port, message } => apply_worker_message(model, port, message),
        AppMessage::WorkerSentInvalid { port, error } => {
            if let Some(worker) = model
                .active_worker_mut()
                .filter(|worker| worker.port == port)
            {
                worker.error = Some(error);
            }
        }
        AppMessage::WorkerDisconnected { port } => {
            model.worker_senders.remove(&port);
            if model.active_port() == Some(port) {
                deactivate_worker(model);
            }
        }
        AppMessage::WorkersDiscovered { workers } => apply_discovered_workers(model, workers),
        AppMessage::Select { selection } => apply_selection(model, selection),
        AppMessage::ShowError { message } => model.overlay = Some(Overlay::Error { message }),
    }
    None
}

fn activate_worker(model: &mut Model, port: u16) {
    model.connection = WorkerConnection::Active(model.fresh_worker(port));
}

fn deactivate_worker(model: &mut Model) {
    let stopped = mem::replace(&mut model.connection, WorkerConnection::Searching);
    if let Some(fallback_port) = model.worker_senders.keys().next().copied() {
        activate_worker(model, fallback_port);
    } else if let WorkerConnection::Active(ActiveWorker {
        port,
        file_uri: Some(file_uri),
        ..
    }) = stopped
    {
        model.connection = WorkerConnection::Stopped { port, file_uri };
    }
}

fn apply_discovered_workers(model: &mut Model, workers: Vec<WorkerInfo>) {
    let new_worker_port = workers
        .iter()
        .map(|worker| worker.port)
        .find(|port| !model.known_workers.iter().any(|known| known.port == *port));
    if let Some(Overlay::WorkerSelector {
        highlighted,
        rescanning,
    }) = &mut model.overlay
    {
        *highlighted = (!workers.is_empty()).then_some(0);
        *rescanning = false;
    }
    model.known_workers = workers;

    if let Some(port) =
        new_worker_port.filter(|port| model.follow_mode && Some(*port) != model.active_port())
    {
        activate_worker(model, port);
    }
}

fn apply_worker_message(model: &mut Model, port: u16, message: ServerToClientMessage) {
    let requests_focus = match &message {
        ServerToClientMessage::FileFocused { focused, .. } => *focused,
        ServerToClientMessage::Cursor { .. } | ServerToClientMessage::Dag { .. } => true,
        _ => false,
    };
    if model.follow_mode && requests_focus && model.active_port() != Some(port) {
        tracing::info!("Auto-switching to port {port} (follow mode)");
        activate_worker(model, port);
    }

    let WorkerConnection::Active(worker) = &mut model.connection else {
        return;
    };
    if worker.port != port {
        return;
    }
    match message {
        ServerToClientMessage::Connected {
            file_uri,
            port_range_min,
            port_range_max,
            editor_name,
            ..
        } => {
            worker.file_uri = file_uri;
            if editor_name.is_some() {
                worker.editor_name = editor_name;
            }
            if let (Some(min), Some(max)) = (port_range_min, port_range_max) {
                model.port_range = u16::try_from(min.get()).unwrap_or(9742)
                    ..=u16::try_from(max.get()).unwrap_or(9752);
            }
        }
        ServerToClientMessage::Cursor {
            uri,
            position,
            method,
        } => {
            worker.dag.update_current_from_cursor(position);
            worker.cursor = Some(EditorCursorPosition {
                method,
                position,
                uri,
            });
            worker.awaiting_graph = true;
        }
        ServerToClientMessage::Dag { dag, .. } => {
            worker.definition = dag.as_ref().and_then(|dag| {
                dag.block_name.clone().map(|name| DefinitionInfo {
                    kind: dag.block_kind.clone(),
                    name,
                })
            });
            worker.awaiting_graph = false;
            worker.dag = match dag.map(DagState::from_generic) {
                Some(Ok(converted)) => {
                    worker.error = None;
                    converted
                }
                Some(Err(error)) => {
                    worker.error = Some(format!("DAG conversion error: {error}"));
                    DagState::None
                }
                None => DagState::None,
            };
            model.display_mode = model.display_mode.compatible_with(&worker.dag);
        }
        ServerToClientMessage::Error { error } => {
            worker.error = Some(error);
            worker.awaiting_graph = false;
        }
        ServerToClientMessage::FileFocused { .. } => {}
    }
}

fn apply_selection(model: &mut Model, selection: Selection) {
    let dag = model.connection.dag();
    model.display_mode.select(selection, dag);
    if let Some(message) = model.display_mode.clicked_error_message(selection, dag) {
        model.overlay = Some(Overlay::Error { message });
    }
}

fn apply_key(model: &mut Model, key: KeyCode) -> Option<Task> {
    if matches!(model.overlay, Some(Overlay::WorkerSelector { .. })) {
        if let Some(action) = find_key_action(WORKER_SELECTOR_KEYBINDINGS, key) {
            return apply_worker_selector_action(model, action);
        }
    }
    if let Some(action) = find_key_action(GLOBAL_KEYBINDINGS, key) {
        return apply_global_action(model, action);
    }
    if let Some(action) = find_key_action(model.display_mode.keybindings(), key) {
        model
            .display_mode
            .apply_action(action, model.connection.dag(), model.timing_display);
    }
    None
}

fn toggle_overlay(model: &mut Model, target: Overlay) {
    let already_open = model
        .overlay
        .as_ref()
        .is_some_and(|current| mem::discriminant(current) == mem::discriminant(&target));
    model.overlay = (!already_open).then_some(target);
}

fn scan_workers_task(model: &Model) -> Task {
    let port_range = model.port_range.clone();
    async move {
        Some(AppMessage::WorkersDiscovered {
            workers: discover_workers(*port_range.start(), *port_range.end()).await,
        })
    }
    .boxed()
}

fn apply_global_action(model: &mut Model, action: GlobalAction) -> Option<Task> {
    match action {
        GlobalAction::Quit => model.running = false,
        GlobalAction::ToggleHelp => toggle_overlay(model, Overlay::Help),
        GlobalAction::ToggleLegend => toggle_overlay(model, Overlay::Legend),
        GlobalAction::ToggleInputPane => model.show_input = !model.show_input,
        GlobalAction::ToggleOutputPane => model.show_output = !model.show_output,
        GlobalAction::OpenWorkerSelector => {
            model.overlay = Some(Overlay::WorkerSelector {
                highlighted: (!model.known_workers.is_empty()).then_some(0),
                rescanning: false,
            });
            return Some(scan_workers_task(model));
        }
        GlobalAction::ToggleFollowMode => model.follow_mode = !model.follow_mode,
        GlobalAction::NextDisplayMode => model.display_mode = model.display_mode.next(),
        GlobalAction::PreviousDisplayMode => model.display_mode = model.display_mode.previous(),
        GlobalAction::GotoSelection => send_navigate_command(model),
        GlobalAction::CopySelection => return copy_selection_task(model),
        GlobalAction::CycleTimingDisplay => model.timing_display.next(),
        GlobalAction::CloseOverlay => model.overlay = None,
    }
    None
}

fn apply_worker_selector_action(model: &mut Model, action: WorkerSelectorAction) -> Option<Task> {
    let worker_count = model.known_workers.len();
    let Some(Overlay::WorkerSelector {
        highlighted,
        rescanning,
    }) = &mut model.overlay
    else {
        return None;
    };
    match action {
        WorkerSelectorAction::HighlightPrevious if worker_count > 0 => {
            *highlighted = Some(highlighted.map_or(0, |index| index.saturating_sub(1)));
        }
        WorkerSelectorAction::HighlightNext if worker_count > 0 => {
            *highlighted = Some(highlighted.map_or(0, |index| (index + 1).min(worker_count - 1)));
        }
        WorkerSelectorAction::HighlightPrevious | WorkerSelectorAction::HighlightNext => {}
        WorkerSelectorAction::Connect => {
            let chosen_port = highlighted
                .and_then(|index| model.known_workers.get(index))
                .map(|worker| worker.port);
            if let Some(port) = chosen_port {
                model.overlay = None;
                model.follow_mode = false;
                if model.active_port() != Some(port) {
                    activate_worker(model, port);
                }
            }
        }
        WorkerSelectorAction::Rescan => {
            *rescanning = true;
            return Some(scan_workers_task(model));
        }
        WorkerSelectorAction::Cancel => model.overlay = None,
    }
    None
}

fn send_navigate_command(model: &Model) {
    let Some(worker) = model.active_worker() else {
        return;
    };
    let Some(cursor) = &worker.cursor else {
        return;
    };
    let goals_position = worker
        .dag
        .proof()
        .and_then(|dag| dag.current_node_id.and_then(|id| dag.get(id)))
        .map(|node| node.position);
    let target = model
        .display_mode
        .current_selection(&worker.dag)
        .and_then(|selection| worker.dag.navigation_location(selection, &cursor.uri))
        .unwrap_or_else(|| FullyQualifiedDocumentPosition {
            uri: cursor.uri.clone(),
            position: goals_position.unwrap_or(cursor.position),
        });
    if let Some(sender) = model.worker_senders.get(&worker.port) {
        let _ = sender.send(ClientToServerCommand::Navigate {
            uri: target.uri,
            position: target.position,
        });
    }
}

fn copy_selection_task(model: &Model) -> Option<Task> {
    let worker = model.active_worker()?;
    let text = worker.error.clone().or_else(|| {
        let dag = worker.dag.proof()?;
        let selection = model.display_mode.current_selection(&worker.dag)?;
        dag.get_selection_text(selection)
    })?;
    Some(
        async move {
            if let Err(error) = stdout().execute(CopyToClipboard::to_clipboard_from(text)) {
                tracing::warn!("Copy to clipboard failed: {error}");
            }
            None
        }
        .boxed(),
    )
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use serde_json::json;
    use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};

    use super::update;
    use crate::{
        message::AppMessage,
        model::{Model, WorkerConnection},
        protocol::{ClientToServerCommand, ServerToClientMessage},
    };

    const FIRST_PORT: u16 = 9742;
    const SECOND_PORT: u16 = 9743;

    fn server_message(value: serde_json::Value) -> ServerToClientMessage {
        serde_json::from_value(value).expect("valid server message")
    }

    fn connect_worker(model: &mut Model, port: u16) -> UnboundedReceiver<ClientToServerCommand> {
        let (commands, received_commands) = unbounded_channel();
        update(model, AppMessage::WorkerConnected { port, commands });
        received_commands
    }

    fn send_from_worker(model: &mut Model, port: u16, message: serde_json::Value) {
        update(
            model,
            AppMessage::WorkerSent {
                port,
                message: server_message(message),
            },
        );
    }

    fn cursor_message(line: u64) -> serde_json::Value {
        json!({
            "type": "Cursor",
            "uri": "file:///project/Basic.lean",
            "position": { "line": line, "character": 2 },
            "method": "hover"
        })
    }

    #[test]
    fn first_connected_worker_becomes_active() {
        let mut model = Model::new(FIRST_PORT..=FIRST_PORT + 10);
        let _commands = connect_worker(&mut model, FIRST_PORT);
        assert_eq!(model.active_port(), Some(FIRST_PORT));
    }

    #[test]
    fn follow_mode_switches_to_worker_whose_file_gains_focus() {
        let mut model = Model::new(FIRST_PORT..=FIRST_PORT + 10);
        let _first = connect_worker(&mut model, FIRST_PORT);
        let _second = connect_worker(&mut model, SECOND_PORT);
        send_from_worker(&mut model, FIRST_PORT, cursor_message(3));

        send_from_worker(
            &mut model,
            SECOND_PORT,
            json!({ "type": "FileFocused", "uri": "file:///project/Other.lean", "focused": true }),
        );

        let active = model.active_worker().expect("active worker");
        assert_eq!(active.port, SECOND_PORT);
        assert!(active.cursor.is_none());
    }

    #[test]
    fn manual_mode_ignores_focus_of_other_worker() {
        let mut model = Model::new(FIRST_PORT..=FIRST_PORT + 10);
        let _first = connect_worker(&mut model, FIRST_PORT);
        let _second = connect_worker(&mut model, SECOND_PORT);
        update(
            &mut model,
            AppMessage::KeyPressed {
                key: KeyEvent::new(KeyCode::Char('F'), KeyModifiers::NONE),
            },
        );

        send_from_worker(&mut model, SECOND_PORT, cursor_message(5));

        assert_eq!(model.active_port(), Some(FIRST_PORT));
    }

    #[test]
    fn active_worker_disconnect_stops_and_reconnect_resumes() {
        let mut model = Model::new(FIRST_PORT..=FIRST_PORT + 10);
        let _commands = connect_worker(&mut model, FIRST_PORT);
        send_from_worker(
            &mut model,
            FIRST_PORT,
            json!({ "type": "Connected", "file_uri": "file:///project/Basic.lean" }),
        );

        update(
            &mut model,
            AppMessage::WorkerDisconnected { port: FIRST_PORT },
        );
        assert!(matches!(
            model.connection,
            WorkerConnection::Stopped {
                port: FIRST_PORT,
                ..
            }
        ));

        let _commands = connect_worker(&mut model, FIRST_PORT);
        assert_eq!(model.active_port(), Some(FIRST_PORT));
    }

    #[test]
    fn goto_sends_navigate_to_active_worker() {
        let mut model = Model::new(FIRST_PORT..=FIRST_PORT + 10);
        let mut commands = connect_worker(&mut model, FIRST_PORT);
        send_from_worker(&mut model, FIRST_PORT, cursor_message(7));

        update(
            &mut model,
            AppMessage::KeyPressed {
                key: KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE),
            },
        );

        assert!(matches!(
            commands.try_recv(),
            Ok(ClientToServerCommand::Navigate { position, .. }) if position.line == 7
        ));
    }

    #[test]
    fn quit_key_stops_running() {
        let mut model = Model::new(FIRST_PORT..=FIRST_PORT + 10);
        update(
            &mut model,
            AppMessage::KeyPressed {
                key: KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE),
            },
        );
        assert!(!model.running);
    }
}
