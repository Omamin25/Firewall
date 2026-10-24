use rusqlite::Connection;

use super::StoreError;

pub fn run(connection: &Connection) -> Result<(), StoreError> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version >= 1 {
        return Ok(());
    }
    connection.execute_batch(
        "BEGIN;
         CREATE TABLE rules (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, action TEXT NOT NULL CHECK (action IN ('block','allow')), direction TEXT NOT NULL CHECK (direction IN ('in','out','both')), protocol TEXT CHECK (protocol IN ('tcp','udp','icmp','any')) DEFAULT 'any', remote_ip TEXT, remote_port INTEGER, local_port INTEGER, process_path TEXT, user_sid TEXT, enabled INTEGER NOT NULL DEFAULT 1, priority INTEGER NOT NULL DEFAULT 100, hit_count INTEGER NOT NULL DEFAULT 0, created_at TEXT NOT NULL, updated_at TEXT NOT NULL);
         CREATE TABLE flows (id INTEGER PRIMARY KEY AUTOINCREMENT, first_seen TEXT NOT NULL, last_seen TEXT NOT NULL, direction TEXT NOT NULL, l7_protocol TEXT, local_ip TEXT NOT NULL, local_port INTEGER, remote_ip TEXT NOT NULL, remote_port INTEGER, remote_host TEXT, protocol TEXT NOT NULL, pid INTEGER, process_path TEXT, user_account TEXT, user_sid TEXT, bytes_sent INTEGER NOT NULL DEFAULT 0, bytes_recv INTEGER NOT NULL DEFAULT 0, packets INTEGER NOT NULL DEFAULT 0, blocked INTEGER NOT NULL DEFAULT 0, matched_rule INTEGER REFERENCES rules(id));
         CREATE INDEX idx_flows_last_seen ON flows(last_seen);
         CREATE INDEX idx_flows_remote_ip ON flows(remote_ip);
         CREATE INDEX idx_flows_pid ON flows(pid);
         CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
         PRAGMA user_version = 1;
         COMMIT;",
    )?;
    Ok(())
}
