use crate::config::SonoraConfig;
use crate::event::SonoraEvent;
use sonora_common::{Result, SonoraError};
use sonora_library::Database;
use std::sync::Arc;
use tokio::sync::broadcast;

/// Primary application context orchestrating subsystems outside the real-time audio path.
pub struct SonoraApp {
    config: SonoraConfig,
    db: Arc<Database>,
    event_tx: broadcast::Sender<SonoraEvent>,
}

impl SonoraApp {
    /// Initialize the application context with configuration and SQLite storage.
    pub fn new(config: SonoraConfig) -> Result<Self> {
        std::fs::create_dir_all(&config.data_dir)
            .map_err(|e| SonoraError::Config(format!("Failed to create data dir: {e}")))?;

        let db_path = config.data_dir.join("library.sqlite3");
        let db = Database::open(&db_path)?;
        let (event_tx, _) = broadcast::channel(256);

        Ok(Self {
            config,
            db: Arc::new(db),
            event_tx,
        })
    }

    /// Initialize an in-memory application context (for tests and headless verification).
    pub fn in_memory(config: SonoraConfig) -> Result<Self> {
        let db = Database::in_memory()?;
        let (event_tx, _) = broadcast::channel(256);

        Ok(Self {
            config,
            db: Arc::new(db),
            event_tx,
        })
    }

    pub fn config(&self) -> &SonoraConfig {
        &self.config
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<SonoraEvent> {
        self.event_tx.subscribe()
    }

    pub fn broadcast_event(&self, event: SonoraEvent) {
        let _ = self.event_tx.send(event);
    }
}
