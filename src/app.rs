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
    modes::{
        BeforeAfterModeInput, DagState, DeductionTreeModeInput, DisplayMode, PlainListInput,
        StepsModeInput,
    },
    protocol::{
        ClientToServerCommand, EditorCursorPosition, LineCharacterPosition, ServerToClientMessage,
        TacticProofState,
    },
    widgets::{
        help_menu::{HelpMenu, HelpMenuWidget},
        status_bar::{StatusBar, StatusBarInput, StatusBarWidget},
        welcome::WelcomeScreen,
        worker_selector::{WorkerInfo, WorkerSelector, WorkerSelectorResult, WorkerSelectorWidget},
        InteractiveStatefulWidget, KeyMouseEvent, Selection,
    },
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
    /// Worker selector dialog.
    worker_selector: WorkerSelector,
    /// File URI of the currently connected worker.
    pub connected_file_uri: Option<String>,
    /// Pending port to switch to (set by worker selector).
    pending_port_switch: Option<u16>,
    /// Whether a port scan is requested.
    pub scan_requested: bool,
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
            worker_selector: WorkerSelector::default(),
            connected_file_uri: None,
            pending_port_switch: None,
            scan_requested: false,
        }
    }

    /// Take a pending port switch request, if any.
    pub fn take_port_switch(&mut self) -> Option<u16> {
        self.pending_port_switch.take()
    }

    /// Clear state when switching to a new worker.
    /// This ensures old data doesn't persist when connecting to a different
    /// file.
    pub fn clear_for_worker_switch(&mut self) {
        self.cursor = None;
        self.definition = None;
        self.error = None;
        self.connected = false;
        self.connected_file_uri = None;
        self.display_mode.dag = DagState::None;
    }

    /// Set discovered workers in the selector.
    pub fn set_discovered_workers(&mut self, workers: Vec<WorkerInfo>) {
        self.worker_selector.set_workers(workers);
        self.scan_requested = false;
    }

    /// Get the port range for scanning.
    pub const fn port_range(&self) -> (u16, u16) {
        self.worker_selector.port_range
    }

    /// Get current proof state from the DAG.
    pub fn proof_state(&self) -> TacticProofState {
        self.display_mode
            .dag
            .proof()
            .and_then(|dag| dag.current_node_id)
            .and_then(|id| self.display_mode.dag.proof()?.get(id))
            .map(|node| node.proof_state_after.clone())
            .unwrap_or_default()
    }

    /// Get position where current goals were fetched.
    pub fn goals_position(&self) -> Option<LineCharacterPosition> {
        self.display_mode
            .dag
            .proof()
            .and_then(|dag| dag.current_node_id)
            .and_then(|id| self.display_mode.dag.proof()?.get(id))
            .map(|node| node.position)
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
                self.error = None;
            }
            ServerToClientMessage::ProofDag { uri, position, dag } => {
                tracing::debug!(
                    "ProofDag: uri={}, pos={}:{}, dag={}",
                    uri,
                    position.line,
                    position.character,
                    dag.as_ref().map_or_else(
                        || "None".to_string(),
                        |d| format!("{} nodes", d.nodes.len())
                    )
                );
                let definition_name = dag.as_ref().and_then(|dag| dag.definition_name.clone());

                self.definition = definition_name.map(|name| DefinitionInfo {
                    name,
                    kind: None,
                    line: None,
                });
                self.display_mode.dag = dag.map_or(DagState::None, DagState::Proof);
                self.connected = true;
                self.error = None;
            }
            ServerToClientMessage::FunctionalDag { uri, position, dag } => {
                tracing::debug!(
                    "FunctionalDag: uri={}, pos={}:{}, dag={}",
                    uri,
                    position.line,
                    position.character,
                    dag.as_ref().map_or_else(
                        || "None".to_string(),
                        |d| format!("{} nodes", d.nodes.len())
                    )
                );
                let definition_name = dag.as_ref().and_then(|dag| dag.definition_name.clone());

                self.definition = definition_name.map(|name| DefinitionInfo {
                    name,
                    kind: Some("def".to_string()),
                    line: None,
                });
                self.display_mode.dag = dag.map_or(DagState::None, DagState::Functional);
                self.connected = true;
                self.error = None;
            }
            ServerToClientMessage::Error { error } => {
                self.error = Some(error);
                self.connected = true;
            }
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

        let dag = self.display_mode.dag.proof();
        let fallback_pos = self.goals_position().unwrap_or(cursor.position);

        // Resolve navigation location based on selection type
        let nav_location: Option<(String, LineCharacterPosition)> =
            selection.and_then(|sel| match sel {
                Selection::InitialHyp { hyp_idx } => dag
                    .and_then(|d| d.initial_proof_state.as_ref())
                    .and_then(|s| s.hypotheses.get(hyp_idx))
                    .and_then(|h| h.navigation_locations().definition)
                    .map(|loc| (loc.uri, loc.position)),
                Selection::Hyp { node_id, hyp_idx } => dag
                    .and_then(|d| d.get(node_id))
                    .and_then(|node| node.proof_state_after.hypotheses.get(hyp_idx))
                    .and_then(|h| h.navigation_locations().definition)
                    .map(|loc| (loc.uri, loc.position)),
                Selection::Goal { node_id, .. } => dag
                    .and_then(|d| d.get(node_id))
                    .map(|node| (cursor.uri.clone(), node.position)),
                Selection::Theorem => None,
            });

        let (uri, position) = nav_location.unwrap_or_else(|| (cursor.uri.clone(), fallback_pos));
        self.queue_command(ClientToServerCommand::Navigate { uri, position });
    }

    /// Get the text of the currently selected item (hypothesis or goal).
    fn get_selection_text(&self, selection: Option<Selection>) -> Option<String> {
        let dag = self.display_mode.dag.proof()?;

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

    /// Copy the selected item's text to the clipboard.
    fn copy_selection_to_clipboard(&self) {
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
            },
        );
    }

    fn update_display_mode(&mut self) {
        let proof_dag = self.display_mode.dag.proof().cloned();
        let functional_dag = self.display_mode.dag.functional().cloned();

        self.display_mode.update_open_goal_list(PlainListInput {
            state: self.proof_state(),
            definition: self.definition.clone(),
            error: self.error.clone(),
            proof_dag: proof_dag.clone(),
        });
        // Derive before/after states from the CompleteProofDag
        let current_node = proof_dag
            .as_ref()
            .and_then(|dag| dag.current_node_id)
            .and_then(|id| proof_dag.as_ref()?.get(id));

        let previous_state = self
            .display_mode
            .show_previous()
            .then(|| current_node.map(|n| n.proof_state_before.clone()))
            .flatten();

        let next_state = self
            .display_mode
            .show_next()
            .then(|| {
                current_node
                    .and_then(|n| n.children.first())
                    .and_then(|&child_id| proof_dag.as_ref()?.get(child_id))
                    .map(|child| child.proof_state_after.clone())
            })
            .flatten();

        self.display_mode.update_before_after(BeforeAfterModeInput {
            previous_state,
            current_state: current_node
                .map(|n| n.proof_state_after.clone())
                .unwrap_or_default(),
            next_state,
            definition: self.definition.clone(),
            error: self.error.clone(),
            proof_dag: proof_dag.clone(),
        });
        self.display_mode.update_steps(StepsModeInput {
            state: self.proof_state(),
            definition: self.definition.clone(),
            error: self.error.clone(),
            proof_dag: proof_dag.clone(),
        });
        self.display_mode
            .update_deduction_tree(DeductionTreeModeInput {
                state: self.proof_state(),
                definition: self.definition.clone(),
                error: self.error.clone(),
                proof_dag,
                functional_dag,
            });
    }

    /// Render the entire UI.
    pub fn render(&mut self, frame: &mut Frame) {
        let [main_area, status_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(1)]).areas(frame.area());

        self.render_main(frame, main_area);
        frame.render_stateful_widget(StatusBarWidget, status_area, &mut self.status_bar);
        frame.render_stateful_widget(HelpMenuWidget, frame.area(), &mut self.help_menu);
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
        if !self.handle_global_key(key.code) {
            self.display_mode.handle_event(KeyMouseEvent::Key(key));
        }
    }

    fn process_worker_selector_result(&mut self) {
        match self.worker_selector.take_result() {
            WorkerSelectorResult::Selected(port) => {
                self.pending_port_switch = Some(port);
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
            KeyCode::Char('w') => {
                // Open worker selector
                self.worker_selector.show();
                self.scan_requested = true;
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
                self.copy_selection_to_clipboard();
                true
            }
            _ => false,
        }
    }
}
