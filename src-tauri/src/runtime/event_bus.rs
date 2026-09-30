use tokio::sync::broadcast;

use crate::domain::events::DomainEvent;

#[derive(Debug, Clone)]
pub struct EventBus {
    sender: broadcast::Sender<DomainEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }
    pub fn publish(&self, event: DomainEvent) -> usize {
        self.sender.send(event).unwrap_or(0)
    }
    pub fn subscribe(&self) -> broadcast::Receiver<DomainEvent> {
        self.sender.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::EventType;
    use serde_json::json;
    #[tokio::test]
    async fn delivers_structured_events_to_subscribers() {
        let bus = EventBus::new(8);
        let mut subscriber = bus.subscribe();
        let event = DomainEvent::new(EventType::AgentCreated, None, json!({ "name": "Atlas" }));
        bus.publish(event.clone());
        assert_eq!(subscriber.recv().await.expect("event"), event);
    }
}
