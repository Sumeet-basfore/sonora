use serde::{Deserialize, Serialize};
use sonora_common::TrackId;
use std::path::PathBuf;

/// An item in the Sonora playback queue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItem {
    pub track_id: Option<TrackId>,
    pub file_path: PathBuf,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub duration_ms: u64,
}

/// Sequential playback queue manager.
#[derive(Debug, Clone, Default)]
pub struct PlaybackQueue {
    items: Vec<QueueItem>,
    current_index: Option<usize>,
}

impl PlaybackQueue {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            current_index: None,
        }
    }

    pub fn enqueue(&mut self, item: QueueItem) {
        self.items.push(item);
        if self.current_index.is_none() && !self.items.is_empty() {
            self.current_index = Some(0);
        }
    }

    pub fn enqueue_many(&mut self, items: Vec<QueueItem>) {
        let was_empty = self.items.is_empty();
        self.items.extend(items);
        if was_empty && !self.items.is_empty() {
            self.current_index = Some(0);
        }
    }

    pub fn insert(&mut self, index: usize, item: QueueItem) {
        let pos = index.min(self.items.len());
        self.items.insert(pos, item);
        if self.current_index.is_none() {
            self.current_index = Some(0);
        }
    }

    pub fn remove(&mut self, index: usize) -> Option<QueueItem> {
        if index < self.items.len() {
            let removed = self.items.remove(index);
            if let Some(curr) = self.current_index {
                if index < curr {
                    self.current_index = Some(curr - 1);
                } else if index == curr {
                    if self.items.is_empty() {
                        self.current_index = None;
                    } else if curr >= self.items.len() {
                        self.current_index = Some(self.items.len() - 1);
                    }
                }
            }
            Some(removed)
        } else {
            None
        }
    }

    pub fn move_item(&mut self, from: usize, to: usize) -> bool {
        if from >= self.items.len() || to >= self.items.len() || from == to {
            return false;
        }
        let item = self.items.remove(from);
        self.items.insert(to, item);
        if let Some(curr) = self.current_index {
            if curr == from {
                self.current_index = Some(to);
            } else if from < curr && to >= curr {
                self.current_index = Some(curr - 1);
            } else if from > curr && to <= curr {
                self.current_index = Some(curr + 1);
            }
        }
        true
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.current_index = None;
    }

    pub fn current(&self) -> Option<&QueueItem> {
        self.current_index.and_then(|idx| self.items.get(idx))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn next(&mut self) -> Option<&QueueItem> {
        if let Some(idx) = self.current_index {
            if idx + 1 < self.items.len() {
                self.current_index = Some(idx + 1);
                return self.current();
            }
        }
        None
    }

    pub fn previous(&mut self) -> Option<&QueueItem> {
        if let Some(idx) = self.current_index {
            if idx > 0 {
                self.current_index = Some(idx - 1);
                return self.current();
            }
        }
        None
    }

    pub fn set_current_index(&mut self, index: usize) -> Option<&QueueItem> {
        if index < self.items.len() {
            self.current_index = Some(index);
            self.current()
        } else {
            None
        }
    }

    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    pub fn items(&self) -> &[QueueItem] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
