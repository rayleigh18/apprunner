//! CRUD operations for the apps and config tables.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use super::models::{AppConfig, NewApp};

/// Insert a new app configuration and return its ID.
pub fn create_app(conn: &Connection, app: &NewApp) -> Result<i64> {
    conn.execute(
        "INSERT INTO apps (name, working_dir, command, env_vars, auto_start, max_runtime_secs) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            app.name,
            app.working_dir,
            app.command,
            app.env_vars,
            app.auto_start,
            app.max_runtime_secs,
        ],
    )
    .context("Failed to insert app")?;
    Ok(conn.last_insert_rowid())
}

/// Retrieve all app configurations.
pub fn get_all_apps(conn: &Connection) -> Result<Vec<AppConfig>> {
    let mut stmt = conn
        .prepare("SELECT id, name, working_dir, command, env_vars, auto_start, max_runtime_secs, created_at FROM apps")
        .context("Failed to prepare get_all_apps query")?;

    let rows = stmt
        .query_map([], |row| {
            Ok(AppConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                working_dir: row.get(2)?,
                command: row.get(3)?,
                env_vars: row.get(4)?,
                auto_start: row.get(5)?,
                max_runtime_secs: row.get(6)?,
                created_at: row.get(7)?,
            })
        })
        .context("Failed to query apps")?;

    let mut apps = Vec::new();
    for row in rows {
        apps.push(row.context("Failed to read app row")?);
    }
    Ok(apps)
}

/// Retrieve a single app configuration by its ID.
pub fn get_app_by_id(conn: &Connection, id: i64) -> Result<AppConfig> {
    conn.query_row(
        "SELECT id, name, working_dir, command, env_vars, auto_start, max_runtime_secs, created_at FROM apps WHERE id = ?1",
        params![id],
        |row| {
            Ok(AppConfig {
                id: row.get(0)?,
                name: row.get(1)?,
                working_dir: row.get(2)?,
                command: row.get(3)?,
                env_vars: row.get(4)?,
                auto_start: row.get(5)?,
                max_runtime_secs: row.get(6)?,
                created_at: row.get(7)?,
            })
        },
    )
    .context(format!("Failed to get app with id {}", id))
}

/// Update an existing app configuration by ID.
pub fn update_app(conn: &Connection, id: i64, app: &NewApp) -> Result<()> {
    let rows_affected = conn
        .execute(
            "UPDATE apps SET name = ?1, working_dir = ?2, command = ?3, env_vars = ?4, auto_start = ?5, max_runtime_secs = ?6 WHERE id = ?7",
            params![
                app.name,
                app.working_dir,
                app.command,
                app.env_vars,
                app.auto_start,
                app.max_runtime_secs,
                id,
            ],
        )
        .context(format!("Failed to update app with id {}", id))?;

    if rows_affected == 0 {
        anyhow::bail!("No app found with id {}", id);
    }
    Ok(())
}

/// Delete an app configuration by ID.
pub fn delete_app(conn: &Connection, id: i64) -> Result<()> {
    let rows_affected = conn
        .execute("DELETE FROM apps WHERE id = ?1", params![id])
        .context(format!("Failed to delete app with id {}", id))?;

    if rows_affected == 0 {
        anyhow::bail!("No app found with id {}", id);
    }
    Ok(())
}

/// Retrieve a config value by key.
pub fn get_config(conn: &Connection, key: &str) -> Result<Option<String>> {
    let result = conn.query_row(
        "SELECT value FROM config WHERE key = ?1",
        params![key],
        |row| row.get(0),
    );

    match result {
        Ok(value) => Ok(Some(value)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context(format!("Failed to get config key '{}'", key)),
    }
}

/// Insert or update a config value.
pub fn set_config(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO config (key, value) VALUES (?1, ?2)",
        params![key, value],
    )
    .context(format!("Failed to set config key '{}'", key))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_memory;

    fn sample_app() -> NewApp {
        NewApp {
            name: "test-app".to_string(),
            working_dir: "/tmp/test".to_string(),
            command: "echo hello".to_string(),
            env_vars: r#"{"PORT":"3000"}"#.to_string(),
            auto_start: false,
            max_runtime_secs: Some(300),
        }
    }

    #[test]
    fn test_create_app_and_retrieve() {
        let conn = init_memory().unwrap();
        let app = sample_app();
        let id = create_app(&conn, &app).unwrap();
        assert!(id > 0);

        let retrieved = get_app_by_id(&conn, id).unwrap();
        assert_eq!(retrieved.name, "test-app");
        assert_eq!(retrieved.working_dir, "/tmp/test");
        assert_eq!(retrieved.command, "echo hello");
        assert_eq!(retrieved.env_vars, r#"{"PORT":"3000"}"#);
        assert!(!retrieved.auto_start);
        assert_eq!(retrieved.max_runtime_secs, Some(300));
    }

    #[test]
    fn test_get_all_apps() {
        let conn = init_memory().unwrap();

        let app1 = NewApp {
            name: "app-one".to_string(),
            working_dir: "/tmp/one".to_string(),
            command: "cmd1".to_string(),
            env_vars: "{}".to_string(),
            auto_start: false,
            max_runtime_secs: None,
        };
        let app2 = NewApp {
            name: "app-two".to_string(),
            working_dir: "/tmp/two".to_string(),
            command: "cmd2".to_string(),
            env_vars: "{}".to_string(),
            auto_start: true,
            max_runtime_secs: Some(60),
        };

        create_app(&conn, &app1).unwrap();
        create_app(&conn, &app2).unwrap();

        let all = get_all_apps(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "app-one");
        assert_eq!(all[1].name, "app-two");
    }

    #[test]
    fn test_update_app() {
        let conn = init_memory().unwrap();
        let app = sample_app();
        let id = create_app(&conn, &app).unwrap();

        let updated = NewApp {
            name: "updated-app".to_string(),
            working_dir: "/tmp/updated".to_string(),
            command: "echo updated".to_string(),
            env_vars: "{}".to_string(),
            auto_start: true,
            max_runtime_secs: None,
        };
        update_app(&conn, id, &updated).unwrap();

        let retrieved = get_app_by_id(&conn, id).unwrap();
        assert_eq!(retrieved.name, "updated-app");
        assert_eq!(retrieved.working_dir, "/tmp/updated");
        assert_eq!(retrieved.command, "echo updated");
        assert!(retrieved.auto_start);
        assert_eq!(retrieved.max_runtime_secs, None);
    }

    #[test]
    fn test_delete_app() {
        let conn = init_memory().unwrap();
        let app = sample_app();
        let id = create_app(&conn, &app).unwrap();

        delete_app(&conn, id).unwrap();

        let result = get_app_by_id(&conn, id);
        assert!(result.is_err());
    }

    #[test]
    fn test_unique_constraint_on_name() {
        let conn = init_memory().unwrap();
        let app = sample_app();
        create_app(&conn, &app).unwrap();

        let duplicate = sample_app();
        let result = create_app(&conn, &duplicate);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_config_set_config() {
        let conn = init_memory().unwrap();

        // Test getting a non-existent key
        let val = get_config(&conn, "nonexistent").unwrap();
        assert_eq!(val, None);

        // Test setting and getting
        set_config(&conn, "my_key", "my_value").unwrap();
        let val = get_config(&conn, "my_key").unwrap();
        assert_eq!(val, Some("my_value".to_string()));

        // Test overwriting
        set_config(&conn, "my_key", "new_value").unwrap();
        let val = get_config(&conn, "my_key").unwrap();
        assert_eq!(val, Some("new_value".to_string()));
    }

    #[test]
    fn test_default_config_value() {
        let conn = init_memory().unwrap();
        let val = get_config(&conn, "global_max_runtime_secs").unwrap();
        assert_eq!(val, Some("18000".to_string()));
    }
}
