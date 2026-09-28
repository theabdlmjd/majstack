use crossbeam_channel::{unbounded, Receiver, Sender};
use majstack_core::{Event, EventKind, Result};
use majstack_state::Store;
use std::sync::{Arc, Mutex};

pub struct EventBus {
    store: Arc<Store>,
    subscribers: Mutex<Vec<Sender<Event>>>,
    run_id: Mutex<Option<String>>,
}

impl EventBus {
    pub fn new(store: Arc<Store>) -> Self {
        EventBus {
            store,
            subscribers: Mutex::new(Vec::new()),
            run_id: Mutex::new(None),
        }
    }

    pub fn subscribe(&self) -> Receiver<Event> {
        let (sender, receiver) = unbounded();
        self.subscribers.lock().unwrap().push(sender);
        receiver
    }

    pub fn set_run(&self, run_id: Option<String>) {
        *self.run_id.lock().unwrap() = run_id;
    }

    pub fn run(&self) -> Option<String> {
        self.run_id.lock().unwrap().clone()
    }

    pub fn emit(&self, kind: EventKind, payload: serde_json::Value) -> Result<Event> {
        let mut event = Event::new(kind, payload);
        if let Some(run) = self.run_id.lock().unwrap().clone() {
            event.run_id = Some(run);
        }
        self.emit_event(event)
    }

    pub fn emit_event(&self, event: Event) -> Result<Event> {
        self.store.append_event(&event)?;
        let subscribers = self.subscribers.lock().unwrap();
        for sender in subscribers.iter() {
            let _ = sender.send(event.clone());
        }
        Ok(event)
    }

    pub fn replay(&self, run_id: &str, limit: i64) -> Result<Vec<Event>> {
        self.store.list_events(run_id, limit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn persists_and_broadcasts() {
        let store = Arc::new(Store::memory().unwrap());
        let run = store.create_run(None, None, None, None, None).unwrap();
        let bus = EventBus::new(store.clone());
        bus.set_run(Some(run.id.clone()));
        let receiver = bus.subscribe();
        bus.emit(EventKind::RunCreated, json!({"x": 1})).unwrap();
        let received = receiver.recv().unwrap();
        assert_eq!(received.kind, EventKind::RunCreated);
        assert_eq!(bus.replay(&run.id, 10).unwrap().len(), 1);
    }
}
