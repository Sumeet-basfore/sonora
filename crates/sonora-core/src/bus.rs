//! Process-local event bus.
//!
//! [`EventBus`] is a thin wrapper over a Tokio broadcast channel. Services
//! publish [`SonoraEvent`]s after mutating their own state; observers
//! (frontends, the plugin bridge, diagnostics) subscribe without holding
//! references to the services themselves.

use crate::event::SonoraEvent;
use tokio::sync::broadcast;

/// Process-local publish/subscribe bus for [`SonoraEvent`].
#[derive(Debug, Clone)]
pub struct EventBus {
    tx: broadcast::Sender<SonoraEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publish an event. A full channel drops the event rather than blocking
    /// a service (slow consumers re-sync via explicit state queries).
    pub fn publish(&self, event: SonoraEvent) {
        let _ = self.tx.send(event);
    }

    /// Subscribe to all future events.
    pub fn subscribe(&self) -> broadcast::Receiver<SonoraEvent> {
        self.tx.subscribe()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}
