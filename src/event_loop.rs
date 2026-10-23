use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    io::Stdout,
    ops::RangeInclusive,
};

use crossterm::event::{Event, EventStream, KeyEventKind, MouseButton, MouseEventKind};
use futures::{stream::FuturesUnordered, StreamExt};
use ratatui::{prelude::CrosstermBackend, Terminal};
use tokio::{
    select,
    sync::mpsc::{unbounded_channel, UnboundedSender},
    task::AbortHandle,
};

use crate::{
    error::Result,
    message::{find_click_target, AppMessage, ClickTarget},
    model::Model,
    tcp_client::{scan_workers_periodically, stay_connected_to_worker},
    update::{update, Task},
    view::view,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SubscriptionKey {
    WorkerSocket { port: u16 },
    WorkerScan { port_range: RangeInclusive<u16> },
}

pub fn subscriptions(model: &Model) -> HashSet<SubscriptionKey> {
    let worker_ports = model
        .known_workers
        .iter()
        .map(|worker| worker.port)
        .chain(model.worker_senders.keys().copied());
    worker_ports
        .map(|port| SubscriptionKey::WorkerSocket { port })
        .chain([SubscriptionKey::WorkerScan {
            port_range: model.port_range.clone(),
        }])
        .collect()
}

fn open_subscription(key: SubscriptionKey, output: UnboundedSender<AppMessage>) -> AbortHandle {
    match key {
        SubscriptionKey::WorkerSocket { port } => {
            tokio::spawn(stay_connected_to_worker(port, output)).abort_handle()
        }
        SubscriptionKey::WorkerScan { port_range } => {
            tokio::spawn(scan_workers_periodically(port_range, output)).abort_handle()
        }
    }
}

fn reconcile_subscriptions(
    running: &mut HashMap<SubscriptionKey, AbortHandle>,
    desired: HashSet<SubscriptionKey>,
    output: &UnboundedSender<AppMessage>,
) {
    running.retain(|key, handle| {
        let keep = desired.contains(key);
        if !keep {
            handle.abort();
        }
        keep
    });
    for key in desired {
        if let Entry::Vacant(vacant) = running.entry(key) {
            let handle = open_subscription(vacant.key().clone(), output.clone());
            vacant.insert(handle);
        }
    }
}

fn terminal_message(event: &Event, click_targets: &[ClickTarget]) -> Option<AppMessage> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            Some(AppMessage::KeyPressed { key: *key })
        }
        Event::Mouse(mouse) if mouse.kind == MouseEventKind::Down(MouseButton::Left) => {
            find_click_target(click_targets, mouse.column, mouse.row)
        }
        _ => None,
    }
}

pub async fn run_event_loop(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    port_range: RangeInclusive<u16>,
) -> Result<()> {
    let mut model = Model::new(port_range);
    let (output, mut app_messages) = unbounded_channel();
    let mut running_subscriptions = HashMap::new();
    let mut terminal_events = EventStream::new();
    let mut pending_tasks = FuturesUnordered::<Task>::new();
    let mut click_targets = Vec::new();

    reconcile_subscriptions(&mut running_subscriptions, subscriptions(&model), &output);
    terminal.draw(|frame| click_targets = view(&model, frame))?;

    while model.running {
        let message = select! {
            Some(Ok(event)) = terminal_events.next() => terminal_message(&event, &click_targets),
            Some(message) = app_messages.recv() => Some(message),
            Some(task_result) = pending_tasks.next() => task_result,
            else => break,
        };
        if let Some(message) = message {
            pending_tasks.extend(update(&mut model, message));
            reconcile_subscriptions(&mut running_subscriptions, subscriptions(&model), &output);
        }
        terminal.draw(|frame| click_targets = view(&model, frame))?;
    }

    running_subscriptions.values().for_each(AbortHandle::abort);
    Ok(())
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc::unbounded_channel;

    use super::{subscriptions, SubscriptionKey};
    use crate::{model::Model, tcp_client::WorkerInfo};

    fn discovered_worker(port: u16) -> WorkerInfo {
        WorkerInfo {
            port,
            file_uri: None,
            server_mode: None,
            editor_name: None,
        }
    }

    #[test]
    fn scans_configured_port_range_without_workers() {
        let model = Model::new(9742..=9752);
        assert_eq!(
            subscriptions(&model),
            [SubscriptionKey::WorkerScan {
                port_range: 9742..=9752
            }]
            .into()
        );
    }

    #[test]
    fn stays_connected_to_discovered_and_connected_workers() {
        let mut model = Model::new(9742..=9752);
        model.known_workers = vec![discovered_worker(9743)];
        let (commands, _pending) = unbounded_channel();
        model.worker_senders.insert(9745, commands);

        let keys = subscriptions(&model);

        assert!(keys.contains(&SubscriptionKey::WorkerSocket { port: 9743 }));
        assert!(keys.contains(&SubscriptionKey::WorkerSocket { port: 9745 }));
        assert_eq!(keys.len(), 3);
    }
}
