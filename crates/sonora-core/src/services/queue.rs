//! Queue service: playback order state.
//!
//! Owns the [`PlaybackQueue`]. Every mutation publishes `QueueChanged` with
//! the new length and cursor, so observers never need the lock themselves.

use crate::bus::EventBus;
use crate::event::SonoraEvent;
use crate::queue::{PlaybackQueue, QueueItem};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct QueueService {
    queue: Arc<Mutex<PlaybackQueue>>,
    bus: EventBus,
}

impl QueueService {
    pub fn new(queue: Arc<Mutex<PlaybackQueue>>, bus: EventBus) -> Self {
        Self { queue, bus }
    }

    fn emit(&self) {
        if let Ok(q) = self.queue.lock() {
            self.bus.publish(SonoraEvent::QueueChanged {
                queue_length: q.len(),
                current_index: q.current_index(),
            });
        }
    }

    pub fn enqueue(&self, item: QueueItem) {
        if let Ok(mut q) = self.queue.lock() {
            q.enqueue(item);
        }
        self.emit();
    }

    pub fn enqueue_many(&self, items: Vec<QueueItem>) {
        if let Ok(mut q) = self.queue.lock() {
            q.enqueue_many(items);
        }
        self.emit();
    }

    pub fn clear(&self) {
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
        }
        self.emit();
    }

    pub fn replace(&self, items: Vec<QueueItem>, current_index: usize) {
        if let Ok(mut q) = self.queue.lock() {
            q.clear();
            q.enqueue_many(items);
            q.set_current_index(current_index);
        }
        self.emit();
    }

    pub fn next(&self) -> Option<QueueItem> {
        let item = self.queue.lock().ok()?.next().cloned();
        self.emit();
        item
    }

    pub fn previous(&self) -> Option<QueueItem> {
        let item = self.queue.lock().ok()?.previous().cloned();
        self.emit();
        item
    }

    pub fn set_current_index(&self, index: usize) -> Option<QueueItem> {
        let item = self.queue.lock().ok()?.set_current_index(index).cloned();
        self.emit();
        item
    }

    pub fn remove(&self, index: usize) -> Option<QueueItem> {
        let item = self.queue.lock().ok()?.remove(index);
        self.emit();
        item
    }

    pub fn move_item(&self, from: usize, to: usize) -> bool {
        let moved = self
            .queue
            .lock()
            .map(|mut q| q.move_item(from, to))
            .unwrap_or(false);
        self.emit();
        moved
    }

    pub fn current(&self) -> Option<QueueItem> {
        self.queue.lock().ok()?.current().cloned()
    }

    pub fn items(&self) -> Vec<QueueItem> {
        self.queue
            .lock()
            .map(|q| q.items().to_vec())
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn current_index(&self) -> Option<usize> {
        self.queue.lock().ok()?.current_index()
    }

    /// Raw handle for orchestration paths that must inspect queue + player
    /// atomically (kept narrow; prefer the methods above).
    pub fn handle(&self) -> &Arc<Mutex<PlaybackQueue>> {
        &self.queue
    }
}
