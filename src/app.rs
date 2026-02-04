//! Application state for the TUI.

use std::{io::stdout, mem};

use crossterm::{
    clipboard::CopyToClipboard,
    event::{Event, KeyCode, KeyEvent, KeyEventKind},
    ExecutableCommand,
};
use ratatui::{
    layout::{Constraint, Layout},
    prelude::*,
    widgets::{Block, Paragraph},
    Frame,
};

use crate::{
    connection::ConnectionPool,
    deduction_tree::ProofContextHypothesis,
    display_mode::{DagState, DisplayMode},
    help_menu::{HelpMenu, HelpMenuWidget},
    protocol::{
        ClientToServerCommand, EditorCursorPosition, LineCharacterPosition, ServerToClientMessage,
    },
    shared::{
        legend::{Legend, LegendWidget},
        InteractiveStatefulWidget, KeyMouseEvent, Selection,
    },
    status_bar::{StatusBar, StatusBarInput, StatusBarWidget},
    welcome::WelcomeScreen,
    worker_selector::{WorkerInfo, WorkerSelector, WorkerSelectorResult, WorkerSelectorWidget},
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
    /// TCP port for lean-dag connection.
    port: u16,
    /// Current cursor position from editor.
    pub cursor: Option<EditorCursorPosition>,
    /// Enclosing definition (theorem/lemma name).
    pub definition: Option<DefinitionInfo>,
    /// Current error message.
    pub error: Option<String>,
    /// Whether connected to lean-dag server.
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
        }
    }

    /// Toggle follow mode on/off.
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

    /// Check if ready for a periodic scan (always scan to detect new workers).
    pub const fn needs_periodic_scan(&self) -> bool {
        !self.worker_selector.scanning
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

    /// Handle incoming message from lean-dag server.
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
                // Update port range if provided
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
                let definition_name = dag.as_ref().and_then(|dag| dag.definition_name.clone());

                self.definition = definition_name.map(|name| DefinitionInfo {
                    name,
                    kind: None,
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
        // Check if we should switch to this port based on the message type
        let should_switch = self.follow_mode
            && match &msg {
                ServerToClientMessage::FileFocused { focused, .. } => *focused,
                ServerToClientMessage::Cursor { .. } => true,
                ServerToClientMessage::Dag { .. } => true,
                _ => false,
            };

        if should_switch && Some(port) != pool.active_port() {
            tracing::info!("Auto-switching to port {} (follow mode)", port);
            pool.set_active(port);
            self.clear_for_worker_switch(port);
        }

        // Only process messages from the active port
        if Some(port) == pool.active_port() {
            self.handle_message(msg);
        }
    }

    /// Cycle to the next display mode.
    pub fn next_mode(&mut self) {
        self.display_mode.next();
    }

    /// Cycle to the previous display mode.
    pub fn prev_mode(&mut self) {
        self.display_mode.prev();
    }

    /// Navigate to where the selected item was introduced.
    pub fn navigate_to_selection(&mut self, selection: Option<Selection>) {
        let Some(cursor) = &self.cursor else {
            return;
        };

        let fallback_pos = self.goals_position().unwrap_or(cursor.position);

        // Resolve navigation location based on selection type
        let nav_location: Option<(String, LineCharacterPosition)> =
            if let DagState::Proof(dag) = &self.display_mode.dag {
                selection.and_then(|sel| match sel {
                    Selection::InitialHyp { hyp_idx } => dag
                        .initial_proof_state
                        .hypotheses
                        .get(hyp_idx)
                        .and_then(|h: &ProofContextHypothesis| h.navigation_locations().definition)
                        .map(|loc| (loc.uri, loc.position)),
                    Selection::Hyp { node_id, hyp_idx } => dag
                        .get(node_id)
                        .and_then(|node| node.proof_state_after.hypotheses.get(hyp_idx))
                        .and_then(|h: &ProofContextHypothesis| h.navigation_locations().definition)
                        .map(|loc| (loc.uri, loc.position)),
                    Selection::Goal { node_id, .. } => dag
                        .get(node_id)
                        .map(|node| (cursor.uri.clone(), node.position)),
                    Selection::Theorem => None,
                })
            } else {
                None
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
                .initial_proof_state()
                .hypotheses
                .get(hyp_idx)
                .map(|h| format!("{} : {}", h.name, h.type_.to_plain_text())),
            Selection::Hyp { node_id, hyp_idx } => dag
                .get(node_id)
                .and_then(|node| node.proof_state_after.hypotheses.get(hyp_idx))
                .map(|h| format!("{} : {}", h.name, h.type_.to_plain_text())),
            Selection::Goal { node_id, goal_idx } => dag
                .get(node_id)
                .and_then(|node| node.proof_state_after.goals.get(goal_idx))
                .map(|g| g.type_.to_plain_text()),
            Selection::Theorem => dag
                .initial_proof_state()
                .goals
                .first()
                .map(|g| g.type_.to_plain_text()),
        }
    }

    /// Copy the selected item's text (or error message) to the clipboard.
    fn copy_to_clipboard(&self) {
        // Prioritize error message if present
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
        self.update_display_mode();
        StatusBarWidget::update_state(
            &mut self.status_bar,
            StatusBarInput {
                filters: self.display_mode.filters(),
                keybindings: self.display_mode.keybindings(),
                supported_filters: self.display_mode.supported_filters(),
                follow_mode: self.follow_mode,
            },
        );
    }

    fn update_display_mode(&mut self) {
        self.display_mode
            .update(self.definition.clone(), self.error.clone());
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
    }

    fn render_main(&mut self, frame: &mut Frame, area: Rect) {
        // Show welcome screen when connected but no data yet
        let has_dag = self.display_mode.dag.is_some();
        let show_welcome = self.connected && self.cursor.is_none() && !has_dag;

        if show_welcome {
            frame.render_widget(WelcomeScreen, area);
            return;
        }

        let title = self.build_title();
        let mode_name = format!(" {} ", self.display_mode.name());
        let backend = self.build_backend_display();
        let position_info = self.build_position_info();

        let block = Block::bordered()
            .title(title)
            .title_top(Line::from(mode_name).right_aligned())
            .title_bottom(Line::from(backend).left_aligned())
            .title_bottom(Line::from(position_info).right_aligned())
            .border_style(Style::new().fg(Color::Cyan));

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
            let kind = def.kind.as_deref().unwrap_or("proof");
            format!(" {kind} {} ({}) ", def.name, filename)
        } else if let Some(cursor) = &self.cursor {
            let filename = cursor.filename().unwrap_or("?");
            format!(" lean-tui [{}] ({}) ", self.display_mode.name(), filename)
        } else {
            format!(" lean-tui [{}] ", self.display_mode.name())
        }
    }

    fn build_position_info(&self) -> String {
        self.cursor.as_ref().map_or(String::new(), |cursor| {
            format!(
                " {}:{} ({}) ",
                cursor.position.line + 1,
                cursor.position.character + 1,
                cursor.method
            )
        })
    }

    fn build_backend_display(&self) -> String {
        if self.display_mode.dag.is_some() {
            " lean-dag ".to_string()
        } else {
            String::new()
        }
    }

    /// Handle crossterm events.
    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                self.handle_key_event(*key);
            }
            Event::Mouse(mouse) => {
                self.display_mode.handle_event(KeyMouseEvent::Mouse(*mouse));
            }
            _ => {}
        }
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
                // Open file selector
                self.worker_selector.show();
                self.scan_requested = true;
                true
            }
            KeyCode::Char('F') => {
                // Toggle follow mode
                self.toggle_follow_mode();
                true
            }
            KeyCode::Char(']') => {
                self.next_mode();
                true
            }
            KeyCode::Char('[') => {
                self.prev_mode();
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
