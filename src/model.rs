use std::{collections::BTreeMap, ops::RangeInclusive};

use tokio::sync::mpsc::UnboundedSender;

use crate::{
    display_mode::{DagState, DisplayMode},
    protocol::{ClientToServerCommand, EditorCursorPosition, TimingDisplay},
    tcp_client::WorkerInfo,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefinitionInfo {
    pub kind: Option<String>,
    pub name: String,
}

pub struct ActiveWorker {
    pub port: u16,
    pub file_uri: Option<String>,
    pub editor_name: Option<String>,
    pub cursor: Option<EditorCursorPosition>,
    pub awaiting_graph: bool,
    pub definition: Option<DefinitionInfo>,
    pub dag: DagState,
    pub error: Option<String>,
}

pub enum WorkerConnection {
    Searching,
    Stopped { port: u16, file_uri: String },
    Active(ActiveWorker),
}

pub enum Overlay {
    Help,
    Legend,
    WorkerSelector {
        highlighted: Option<usize>,
        rescanning: bool,
    },
    Error {
        message: String,
    },
}

pub struct Model {
    pub running: bool,
    pub worker_senders: BTreeMap<u16, UnboundedSender<ClientToServerCommand>>,
    pub known_workers: Vec<WorkerInfo>,
    pub port_range: RangeInclusive<u16>,
    pub connection: WorkerConnection,
    pub follow_mode: bool,
    pub show_input: bool,
    pub show_output: bool,
    pub overlay: Option<Overlay>,
    pub display_mode: DisplayMode,
    pub timing_display: TimingDisplay,
}

static NO_DAG: DagState = DagState::None;

impl WorkerConnection {
    pub fn dag(&self) -> &DagState {
        match self {
            Self::Active(worker) => &worker.dag,
            _ => &NO_DAG,
        }
    }
}

impl Model {
    pub fn new(port_range: RangeInclusive<u16>) -> Self {
        Self {
            running: true,
            worker_senders: BTreeMap::new(),
            known_workers: Vec::new(),
            port_range,
            connection: WorkerConnection::Searching,
            follow_mode: true,
            show_input: false,
            show_output: false,
            overlay: None,
            display_mode: DisplayMode::default(),
            timing_display: TimingDisplay::Off,
        }
    }

    pub const fn active_worker(&self) -> Option<&ActiveWorker> {
        match &self.connection {
            WorkerConnection::Active(worker) => Some(worker),
            _ => None,
        }
    }

    pub const fn active_worker_mut(&mut self) -> Option<&mut ActiveWorker> {
        match &mut self.connection {
            WorkerConnection::Active(worker) => Some(worker),
            _ => None,
        }
    }

    pub fn active_port(&self) -> Option<u16> {
        self.active_worker().map(|worker| worker.port)
    }

    pub fn dag(&self) -> &DagState {
        self.connection.dag()
    }

    pub fn fresh_worker(&self, port: u16) -> ActiveWorker {
        let info = self.known_workers.iter().find(|worker| worker.port == port);
        ActiveWorker {
            port,
            file_uri: info.and_then(|worker| worker.file_uri.clone()),
            editor_name: info.and_then(|worker| worker.editor_name.clone()),
            cursor: None,
            awaiting_graph: false,
            definition: None,
            dag: DagState::None,
            error: None,
        }
    }
}
