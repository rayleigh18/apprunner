//! SQLite database operations for app configuration storage.

use anyhow::{Context, Result};
use rusqlite::Connection;

pub mod models;
pub mod operations;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS apps (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    working_dir TEXT NOT NULL,
    command TEXT NOT NULL,
    env_vars TEXT DEFAULT '{}',
    auto_start BOOLEAN DEFAULT 0,
    max_runtime_secs INTEGER DEFAULT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS config (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
";

/// Run schema creation and insert default config values.
fn setup_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(SCHEMA)
        .context("Failed to create database schema")?;

    // Insert default config if not already present
    conn.execute(
        "INSERT OR IGNORE INTO config (key, value) VALUES ('global_max_runtime_secs', '18000')",
        [],
    )
    .context("Failed to insert default config")?;

    Ok(())
}

/// Initialize the database connection and create tables if needed.
/// DB file is stored at `~/.local/share/apprunner/apprunner.db`.
pub fn init() -> Result<Connection> {
    let data_dir = dirs::data_local_dir()
        .context("Failed to determine local data directory")?
        .join("apprunner");

    std::fs::create_dir_all(&data_dir)
        .context(format!("Failed to create data directory: {:?}", data_dir))?;

    let db_path = data_dir.join("apprunner.db");
    let conn =
        Connection::open(&db_path).context(format!("Failed to open database at {:?}", db_path))?;

    setup_schema(&conn)?;
    Ok(conn)
}

/// Initialize an in-memory database for testing.
pub fn init_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory().context("Failed to open in-memory database")?;
    setup_schema(&conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_creation() {
        let conn = init_memory().unwrap();

        // Verify apps table exists by querying it
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM apps", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 0);

        // Verify config table exists
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM config", [], |row| row.get(0))
            .unwrap();
        assert!(count >= 1); // At least the default config
    }

    #[test]
    fn test_init_memory_is_idempotent() {
        // Calling init_memory twice should work (CREATE IF NOT EXISTS)
        let conn = init_memory().unwrap();
        // Run schema again on the same connection
        setup_schema(&conn).unwrap();
    }
}
