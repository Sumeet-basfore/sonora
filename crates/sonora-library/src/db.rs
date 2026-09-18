use crate::schema::SCHEMA_SQL;
use rusqlite::Connection;
use sonora_common::{Result, SonoraError};
use std::path::Path;
use std::sync::{Mutex, MutexGuard};

/// Thread-safe SQLite database connection and configuration manager.
pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    /// Open a file-backed SQLite database, configure WAL mode and pragmas, and run schema migrations.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path.as_ref())
            .map_err(|e| SonoraError::Database(format!("Failed to open SQLite database: {e}")))?;

        Self::configure(&conn)?;
        Self::migrate(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    /// Open an in-memory SQLite database (primarily for testing and benchmarks).
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory().map_err(|e| {
            SonoraError::Database(format!("Failed to open in-memory database: {e}"))
        })?;

        Self::configure(&conn)?;
        Self::migrate(&conn)?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn configure(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -64000;
            PRAGMA foreign_keys = ON;
            "#,
        )
        .map_err(|e| SonoraError::Database(format!("Failed to configure SQLite pragmas: {e}")))?;

        Ok(())
    }

    fn migrate(conn: &Connection) -> Result<()> {
        conn.execute_batch(SCHEMA_SQL)
            .map_err(|e| SonoraError::Database(format!("Failed to apply schema: {e}")))?;

        Ok(())
    }

    pub fn lock_conn(&self) -> Result<MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|e| SonoraError::Database(format!("Mutex poison error: {e}")))
    }
}
