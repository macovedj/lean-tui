//! Application state for the TUI.

use std::{io::stdout, mem};

use crossterm::{
    clipboard::CopyToClipboard,
    event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Layout},
    prelude::*,
    widgets::{Block, Paragraph},
    Frame,
};

use crate::{
    data_flow::DataFlowNode,
    deduction_tree::ProofContextHypothesis,
    display_mode::{DagState, DisplayMode},
    help_menu::{HelpMenu, HelpMenuWidget},
    protocol::{
        ClientToServerCommand, EditorCursorPosition, LineCharacterPosition, RichText,
        ServerToClientMessage,
    },
    shared::{
        legend::{Legend, LegendWidget},
        theme::Theme,
        InteractiveStatefulWidget, KeyMouseEvent, Selection,
    },
    status_bar::{StatusBar, StatusBarInput, StatusBarWidget},
    tcp_client::{ConnectionPool, WorkerInfo},
    welcome::WelcomeScreen,
    worker_selector::{WorkerSelector, WorkerSelectorResult, WorkerSelectorWidget},
};

/// Information about the enclosing definition (theorem, lemma, def, etc.)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionInfo {
    /// Kind of definition (theorem, lemma, def, example)
    pub kind: Option<String>,
    /// Name of the definition
    pub name: String,
    /// Line where the definition starts
    pub line: Option<u32>,
}

/// Application state.
pub struct App {
    /// TCP port for lean-prism connection.
    port: u16,
    /// Current cursor position from editor.
    pub cursor: Option<EditorCursorPosition>,
    /// Enclosing definition (theorem/lemma name).
    pub definition: Option<DefinitionInfo>,
    /// Current error message.
    pub error: Option<String>,
    /// Whether connected to lean-prism server.
    pub connected: bool,
    /// Whether app should exit.
    pub should_exit: bool,
    /// Outgoing commands queue.
    outgoing_commands: Vec<ClientToServerCommand>,
    /// Current display mode (includes DAG state).
    display_mode: DisplayMode,
    /// Status bar component.
    status_bar: StatusBar,
    /// Help menu overlay.
    help_menu: HelpMenu,
    /// Legend popup overlay.
    legend: Legend,
    /// Worker selector dialog.
    worker_selector: WorkerSelector,
    /// File URI of the currently connected worker.
    pub connected_file_uri: Option<String>,
    /// Pending port to switch to (set by worker selector).
    pending_port_switch: Option<u16>,
    /// Whether a port scan is requested.
    pub scan_requested: bool,
    /// Previously known worker ports (for detecting new workers).
    known_worker_ports: Vec<u16>,
    /// Whether to auto-follow file switches from the editor.
    follow_mode: bool,
    /// Error message to show as overlay (from clicking on error in graph).
    error_overlay: Option<String>,
}

impl App {
    /// Create a new App with the given TCP port.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            cursor: None,
            definition: None,
            error: None,
            connected: false,
            should_exit: false,
            outgoing_commands: Vec::new(),
            display_mode: DisplayMode::default(),
            status_bar: StatusBar::default(),
            help_menu: HelpMenu::default(),
            legend: Legend::default(),
            worker_selector: WorkerSelector::default(),
            connected_file_uri: None,
            pending_port_switch: None,
            scan_requested: true, // Trigger initial scan on startup
            known_worker_ports: Vec::new(),
            follow_mode: true,
            error_overlay: None,
        }
    }

    pub fn toggle_follow_mode(&mut self) {
        self.follow_mode = !self.follow_mode;
    }

    /// Take a pending port switch request, if any.
    pub fn take_port_switch(&mut self) -> Option<u16> {
        self.pending_port_switch.take()
    }

    /// Clear state when switching to a new worker.
    /// This ensures old data doesn't persist when connecting to a different
    /// file.
    pub fn clear_for_worker_switch(&mut self, new_port: u16) {
        self.port = new_port;
        self.cursor = None;
        self.definition = None;
        self.error = None;
        self.connected = false;
        self.connected_file_uri = None;
        self.display_mode.dag = DagState::None;
    }

    /// Set discovered workers in the selector.
    /// Auto-switches to newly appeared workers when follow mode is enabled.
    pub fn set_discovered_workers(&mut self, workers: Vec<WorkerInfo>, pool: &ConnectionPool) {
        let new_worker_port = workers
            .iter()
            .find(|w| !self.known_worker_ports.contains(&w.port))
            .map(|w| w.port);

        self.known_worker_ports = workers.iter().map(|w| w.port).collect();
        self.worker_selector.set_workers(workers);
        self.scan_requested = false;

        // Auto-switch to new workers only when follow mode is enabled
        let should_switch = self.follow_mode
            && new_worker_port.is_some_and(|port| Some(port) != pool.active_port());

        if should_switch {
            self.pending_port_switch = new_worker_port;
        }
    }

    /// Get the port range for scanning.
    pub const fn port_range(&self) -> (u16, u16) {
        self.worker_selector.port_range
    }

    /// Get position where current goals were fetched.
    pub fn goals_position(&self) -> Option<LineCharacterPosition> {
        let DagState::Proof(dag) = &self.display_mode.dag else {
            return None;
        };
        dag.current_node_id
            .and_then(|id| dag.get(id))
            .map(|node| node.position)
    }

    /// Set an error message to be displayed in the UI.
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
    }

    /// Queue a command to be sent to the server.
    pub fn queue_command(&mut self, cmd: ClientToServerCommand) {
        self.outgoing_commands.push(cmd);
    }

    /// Take all queued commands.
    pub fn take_commands(&mut self) -> Vec<ClientToServerCommand> {
        mem::take(&mut self.outgoing_commands)
    }

    /// Handle incoming message from lean-prism server.
    pub fn handle_message(&mut self, msg: ServerToClientMessage) {
        match msg {
            ServerToClientMessage::Connected {
                file_uri,
                port_range_min,
                port_range_max,
                ..
            } => {
                self.connected = true;
                self.connected_file_uri = file_uri;
                if let (Some(min), Some(max)) = (port_range_min, port_range_max) {
                    let min_port = u16::try_from(min.get()).unwrap_or(9742);
                    let max_port = u16::try_from(max.get()).unwrap_or(9752);
                    self.worker_selector.set_port_range(min_port, max_port);
                }
            }
            ServerToClientMessage::Cursor {
                uri,
                position,
                method,
            } => {
                tracing::debug!(
                    "Cursor: uri={}, pos={}:{}, method={}",
                    uri,
                    position.line,
                    position.character,
                    method
                );
                self.cursor = Some(EditorCursorPosition {
                    method,
                    position,
                    uri,
                });
                self.connected = true;
            }
            ServerToClientMessage::Dag { uri, position, dag } => {
                tracing::debug!(
                    "Dag: uri={}, pos={}:{}, dag={}, current_node_id={:?}",
                    uri,
                    position.line,
                    position.character,
                    dag.as_ref().map_or_else(
                        || "None".to_string(),
                        |d| format!("{} nodes, style={:?}", d.nodes.len(), d.display_style)
                    ),
                    dag.as_ref().and_then(|d| d.current_node_id)
                );
                let block_name = dag.as_ref().and_then(|dag| dag.block_name.clone());
                let block_kind = dag.as_ref().and_then(|dag| dag.block_kind.clone());

                self.definition = block_name.map(|name| DefinitionInfo {
                    name,
                    kind: block_kind,
                    line: None,
                });
                self.display_mode.dag = match dag {
                    Some(d) => match DagState::from_generic(d) {
                        Ok(state) => {
                            self.error = None;
                            self.display_mode.switch_to_compatible_mode(&state);
                            state
                        }
                        Err(e) => {
                            self.error = Some(format!("DAG conversion error: {e}"));
                            DagState::None
                        }
                    },
                    None => DagState::None,
                };
                self.connected = true;
            }
            ServerToClientMessage::Error { error } => {
                self.error = Some(error);
                self.connected = true;
            }
            ServerToClientMessage::FileFocused { uri, focused } => {
                tracing::debug!("FileFocused: uri={}, focused={}", uri, focused);
                // FileFocused is handled in handle_pool_message for port
                // switching
            }
        }
    }

    /// Handle a message from the connection pool, potentially switching active
    /// port.
    pub fn handle_pool_message(
        &mut self,
        port: u16,
        msg: ServerToClientMessage,
        pool: &mut ConnectionPool,
    ) {
        let should_switch = self.follow_mode
            && match &msg {
                ServerToClientMessage::FileFocused { focused, .. } => *focused,
                ServerToClientMessage::Cursor { .. } | ServerToClientMessage::Dag { .. } => true,
                _ => false,
            };

        if should_switch && Some(port) != pool.active_port() {
            tracing::info!("Auto-switching to port {} (follow mode)", port);
            pool.set_active(port);
            self.clear_for_worker_switch(port);
        }

        if Some(port) == pool.active_port() {
            self.handle_message(msg);
        }
    }

    /// Navigate to where the selected item was introduced.
    pub fn navigate_to_selection(&mut self, selection: Option<Selection>) {
        let Some(cursor) = &self.cursor else {
            return;
        };

        let fallback_pos = self.goals_position().unwrap_or(cursor.position);

        let nav_location: Option<(String, LineCharacterPosition)> = match &self.display_mode.dag {
            DagState::Proof(dag) => selection.and_then(|sel| match sel {
                Selection::InitialHyp { hyp_idx } => dag
                    .block_input
                    .get(hyp_idx)
                    .and_then(|b| b.navigation_locations.as_ref())
                    .and_then(|nav| nav.definition.as_ref())
                    .map(|loc| (loc.uri.clone(), loc.position)),
                Selection::Hyp { node_id, hyp_idx } => dag
                    .get(node_id)
                    .and_then(|node| node.proof_state.hypotheses.get(hyp_idx))
                    .and_then(|h: &ProofContextHypothesis| h.navigation_locations().definition)
                    .map(|loc| (loc.uri, loc.position)),
                Selection::Goal { node_id, .. } => dag
                    .get(node_id)
                    .map(|node| (cursor.uri.clone(), node.position)),
                Selection::Theorem | Selection::Error { .. } => None,
            }),
            DagState::Dataflow(dag) => selection.and_then(|sel| match sel {
                Selection::InitialHyp { hyp_idx } => dag
                    .block_input
                    .get(hyp_idx)
                    .and_then(|b| b.navigation_locations.as_ref())
                    .and_then(|nav| nav.definition.as_ref())
                    .map(|loc| (loc.uri.clone(), loc.position)),
                Selection::Hyp { node_id, hyp_idx } => dag
                    .get(node_id)
                    .and_then(|node: &DataFlowNode| node.bindings_after().get(hyp_idx).cloned())
                    .and_then(|b| b.navigation_locations)
                    .and_then(|nav| nav.definition)
                    .map(|loc| (loc.uri, loc.position)),
                Selection::Goal { node_id, .. } => dag
                    .get(node_id)
                    .map(|node| (cursor.uri.clone(), node.position)),
                Selection::Theorem => dag
                    .root_node_id
                    .and_then(|id| dag.get(id))
                    .map(|node| (cursor.uri.clone(), node.position)),
                Selection::Error { .. } => None,
            }),
            DagState::Effectflow(_) | DagState::None => None,
        };

        let (uri, position) = nav_location.unwrap_or_else(|| (cursor.uri.clone(), fallback_pos));
        self.queue_command(ClientToServerCommand::Navigate { uri, position });
    }

    /// Get the text of the currently selected item (hypothesis or goal).
    fn get_selection_text(&self, selection: Option<Selection>) -> Option<String> {
        let DagState::Proof(dag) = &self.display_mode.dag else {
            return None;
        };

        match selection? {
            Selection::InitialHyp { hyp_idx } => dag
                .block_input
                .get(hyp_idx)
                .map(|b| format!("{} : {}", b.name, b.type_.to_plain_text())),
            Selection::Hyp { node_id, hyp_idx } => dag
                .get(node_id)
                .and_then(|node| node.proof_state.hypotheses.get(hyp_idx))
                .map(|h| format!("{} : {}", h.name, h.type_.to_plain_text())),
            Selection::Goal { node_id, goal_idx } => dag
                .get(node_id)
                .and_then(|node| {
                    node.proof_state
                        .goals
                        .iter()
                        .filter(|g| !g.is_removed)
                        .nth(goal_idx)
                })
                .map(|g| g.type_.to_plain_text()),
            Selection::Theorem => dag.block_output.as_ref().map(RichText::to_plain_text),
            Selection::Error { node_id, error_idx } => {
                use crate::protocol::DiagnosticSeverity;
                dag.get(node_id).and_then(|node| {
                    node.diagnostics
                        .iter()
                        .filter(|d| d.severity == DiagnosticSeverity::Error)
                        .nth(error_idx)
                        .map(|d| d.message.clone())
                })
            }
        }
    }

    /// Copy the selected item's text (or error message) to the clipboard.
    fn copy_to_clipboard(&self) {
        if let Some(error) = &self.error {
            let _ = stdout().execute(CopyToClipboard::to_clipboard_from(error.clone()));
            return;
        }

        let selection = self.display_mode.current_selection();
        if let Some(text) = self.get_selection_text(selection) {
            let _ = stdout().execute(CopyToClipboard::to_clipboard_from(text));
        }
    }

    /// Update all components with current state.
    pub fn update(&mut self) {
        self.display_mode
            .update(self.definition.clone(), self.error.clone());
        StatusBarWidget::update_state(
            &mut self.status_bar,
            StatusBarInput {
                filters: self.display_mode.filters(),
                keybindings: self.display_mode.keybindings(),
                supported_filters: self.display_mode.supported_filters(),
            },
        );
    }

    /// Render the entire UI.
    pub fn render(&mut self, frame: &mut Frame) {
        let [main_area, status_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        self.render_main(frame, main_area);
        frame.render_stateful_widget(StatusBarWidget, status_area, &mut self.status_bar);
        frame.render_stateful_widget(HelpMenuWidget, frame.area(), &mut self.help_menu);
        let legend_sections = self.display_mode.legend_sections();
        frame.render_stateful_widget(
            LegendWidget::new(&legend_sections),
            frame.area(),
            &mut self.legend,
        );
        frame.render_stateful_widget(
            WorkerSelectorWidget,
            frame.area(),
            &mut self.worker_selector,
        );

        // Render error overlay on top of everything
        if let Some(ref error_msg) = self.error_overlay {
            use ratatui::widgets::Clear;

            use crate::shared::error_banner::ErrorBanner;

            let banner = ErrorBanner::new(error_msg);
            let height = banner
                .required_height(main_area.width)
                .min(main_area.height / 2);
            let overlay_area = Rect::new(main_area.x, main_area.y, main_area.width, height);
            frame.render_widget(Clear, overlay_area);
            frame.render_widget(banner, overlay_area);
        }
    }

    fn render_main(&mut self, frame: &mut Frame, area: Rect) {
        let has_dag = self.display_mode.dag.is_some();
        let show_welcome = self.connected && self.cursor.is_none() && !has_dag;

        if show_welcome {
            frame.render_widget(WelcomeScreen, area);
            return;
        }

        let title = self.build_title();
        let mode_name = format!(" {} ", self.display_mode.name());
        let position_info = self.build_position_info();

        let block = Block::bordered()
            .title(title)
            .title_top(Line::from(mode_name).right_aligned())
            .title_bottom(Line::from(position_info).right_aligned())
            .border_style(Style::new().fg(Theme::FRAME_BORDER));

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if !self.connected {
            frame.render_widget(
                Paragraph::new(format!("Connecting to 127.0.0.1:{}...", self.port)),
                inner,
            );
            return;
        }

        self.display_mode.render(frame, inner);
    }

    fn build_title(&self) -> String {
        if let (Some(def), Some(cursor)) = (&self.definition, &self.cursor) {
            let filename = cursor.filename().unwrap_or("?");
            def.kind.as_ref().map_or_else(
                || format!(" {} ({}) ", def.name, filename),
                |kind| format!(" {kind} {} ({}) ", def.name, filename),
            )
        } else if let Some(cursor) = &self.cursor {
            let filename = cursor.filename().unwrap_or("?");
            format!("lean-tui ({filename}) ")
        } else {
            "lean-tui".to_string()
        }
    }

    fn build_position_info(&self) -> String {
        let follow = if self.follow_mode { "follow" } else { "manual" };
        self.cursor.as_ref().map_or_else(
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

    /// Handle crossterm events.
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.handle_key_event(*key);
            }
            Event::Mouse(mouse) => self.handle_mouse_event(*mouse),
            _ => {}
        }
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) {
        use crossterm::event::{MouseButton, MouseEventKind};

        let is_click = mouse.kind == MouseEventKind::Down(MouseButton::Left);
        self.display_mode.handle_event(KeyMouseEvent::Mouse(mouse));

        if !is_click {
            return;
        }

        self.update_error_overlay_from_selection();
        if let Some(error_msg) = self.display_mode.take_clicked_error() {
            self.error_overlay = Some(error_msg);
        }
    }

    /// Update `error_overlay` based on current selection.
    fn update_error_overlay_from_selection(&mut self) {
        use crate::protocol::DiagnosticSeverity;

        let Some(Selection::Error { node_id, error_idx }) = self.display_mode.current_selection()
        else {
            return;
        };

        let DagState::Proof(dag) = &self.display_mode.dag else {
            return;
        };

        self.error_overlay = dag
            .get(node_id)
            .and_then(|node| {
                node.diagnostics
                    .iter()
                    .filter(|d| d.severity == DiagnosticSeverity::Error)
                    .nth(error_idx)
            })
            .map(|d| d.message.clone());
    }

    fn handle_key_event(&mut self, key: KeyEvent) {
        // Worker selector takes priority when visible
        if WorkerSelectorWidget::handle_event(&mut self.worker_selector, key) {
            self.process_worker_selector_result();
            return;
        }
        if HelpMenuWidget::handle_event(&mut self.help_menu, key) {
            return;
        }
        if LegendWidget::handle_event(&mut self.legend, key) {
            return;
        }
        if !self.handle_global_key(key.code) {
            self.display_mode.handle_event(KeyMouseEvent::Key(key));
        }
    }

    fn process_worker_selector_result(&mut self) {
        match self.worker_selector.take_result() {
            WorkerSelectorResult::Selected(port) => {
                self.pending_port_switch = Some(port);
                // Manual selection disables follow mode
                self.follow_mode = false;
            }
            WorkerSelectorResult::Rescan => {
                self.scan_requested = true;
            }
            _ => {}
        }
    }

    fn handle_global_key(&mut self, code: KeyCode) -> bool {
        match code {
            KeyCode::Esc if self.error_overlay.is_some() => {
                self.error_overlay = None;
                true
            }
            KeyCode::Char('q') => {
                self.should_exit = true;
                true
            }
            KeyCode::Char('?') => {
                self.help_menu.toggle();
                true
            }
            KeyCode::Char('L') => {
                self.legend.toggle();
                true
            }
            KeyCode::Char('f') => {
                self.worker_selector.show();
                self.scan_requested = true;
                true
            }
            KeyCode::Char('F') => {
                self.toggle_follow_mode();
                true
            }
            KeyCode::Char(']') => {
                self.display_mode.next();
                true
            }
            KeyCode::Char('[') => {
                self.display_mode.prev();
                true
            }
            KeyCode::Char('g') | KeyCode::Enter => {
                let selection = self.display_mode.current_selection();
                self.navigate_to_selection(selection);
                true
            }
            KeyCode::Char('y') => {
                self.copy_to_clipboard();
                true
            }
            _ => false,
        }
    }
}
