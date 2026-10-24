mod flows;
mod migrations;
mod rules;

use std::{
    fs,
    path::{Path, PathBuf},
    sync::Mutex,
    sync::MutexGuard,
};

use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

pub use flows::{FlowHistorySummary, RetentionPolicy, DEFAULT_MAX_ROWS, DEFAULT_RETENTION_HOURS};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("database lock was poisoned")]
    LockPoisoned,
    #[error("rule {0} no longer exists")]
    RuleNotFound(i64),
    #[error("a stored value could not be read: {0}")]
    Malformed(String),
    #[error(transparent)]
    Rule(#[from] crate::model::RuleFieldError),
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

pub struct Store {
    connection: Mutex<Connection>,
    /// Where the database actually lives, so diagnostics can show the real location
    /// (including the fallback directory) rather than a hard-coded file name.
    path: PathBuf,
}

impl Store {
    pub fn open(app_data_dir: &Path) -> Result<Self, StoreError> {
        fs::create_dir_all(app_data_dir)?;
        let path = app_data_dir.join("cognton.sqlite3");
        let connection = Connection::open(&path)?;
        migrations::run(&connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
            path,
        })
    }

    /// Absolute path of the open database file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Single guarded connection (`library_docs.md` §6). Callers hold the guard for one
    /// statement or one transaction, never across other work.
    fn connection(&self) -> Result<MutexGuard<'_, Connection>, StoreError> {
        self.connection.lock().map_err(|_| StoreError::LockPoisoned)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), StoreError> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, StoreError> {
        let connection = self.connection()?;
        let mut statement = connection.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let value = statement
            .query_row(params![key], |row| row.get(0))
            .optional()?;
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Store;

    #[test]
    fn runs_migrations_and_round_trips_a_setting() {
        let directory = tempfile::tempdir().expect("test temp directory should be created");
        let store = Store::open(directory.path()).expect("store should open");
        store
            .set_setting("phase_one", "ready")
            .expect("setting should save");
        assert_eq!(
            store.get_setting("phase_one").expect("setting should load"),
            Some("ready".to_owned())
        );
    }
}
