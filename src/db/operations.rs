//! CRUD operations for the apps and config tables.

use anyhow::{Context, Result};
use rusqlite::{params, Connection};

use super::models::{ApiMask, AppConfig, NewApp, NewMask};
use crate::template::{parse_template_vars, serialize_template_vars};

/// Insert a new app configuration and return its ID.
pub fn create_app(conn: &Connection, app: &NewApp) -> Result<i64> {
    let template_vars_json = serialize_template_vars(&app.template_vars)?;
    conn.execute(
        "INSERT INTO apps (name, working_dir, command, env_vars, auto_start, max_runtime_secs, interval_seconds, template_vars) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            app.name,
            app.working_dir,
            app.command,
            app.env_vars,
            app.auto_start,
            app.max_runtime_secs,
            app.interval_seconds,
            template_vars_json,
        ],
    )
    .context("Failed to insert app")?;
    Ok(conn.last_insert_rowid())
}

/// Retrieve all app configurations.
pub fn get_all_apps(conn: &Connection) -> Result<Vec<AppConfig>> {
    let mut stmt = conn
        .prepare("SELECT id, name, working_dir, command, env_vars, auto_start, max_runtime_secs, interval_seconds, template_vars, created_at FROM apps")
        .context("Failed to prepare get_all_apps query")?;

    let rows = stmt
        .query_map([], |row| {
            let template_vars_json: String = row.get(8)?;
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                template_vars_json,
                row.get(9)?,
            ))
        })
        .context("Failed to query apps")?;

    let mut apps = Vec::new();
    for row in rows {
        let (id, name, working_dir, command, env_vars, auto_start, max_runtime_secs, interval_seconds, template_vars_json, created_at): (i64, String, String, String, String, bool, Option<i64>, Option<i64>, String, String) =
            row.context("Failed to read app row")?;
        let template_vars = parse_template_vars(&template_vars_json)
            .unwrap_or_default();
        apps.push(AppConfig {
            id,
            name,
            working_dir,
            command,
            env_vars,
            auto_start,
            max_runtime_secs,
            interval_seconds,
            template_vars,
            created_at,
        });
    }
    Ok(apps)
}

/// Retrieve a single app configuration by its ID.
pub fn get_app_by_id(conn: &Connection, id: i64) -> Result<AppConfig> {
    let row = conn.query_row(
        "SELECT id, name, working_dir, command, env_vars, auto_start, max_runtime_secs, interval_seconds, template_vars, created_at FROM apps WHERE id = ?1",
        params![id],
        |row| {
            let template_vars_json: String = row.get(8)?;
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                template_vars_json,
                row.get(9)?,
            ))
        },
    )
    .context(format!("Failed to get app with id {}", id))?;

    let (id, name, working_dir, command, env_vars, auto_start, max_runtime_secs, interval_seconds, template_vars_json, created_at): (i64, String, String, String, String, bool, Option<i64>, Option<i64>, String, String) = row;
    let template_vars = parse_template_vars(&template_vars_json)
        .unwrap_or_default();

    Ok(AppConfig {
        id,
        name,
        working_dir,
        command,
        env_vars,
        auto_start,
        max_runtime_secs,
        interval_seconds,
        template_vars,
        created_at,
    })
}

/// Update an existing app configuration by ID.
pub fn update_app(conn: &Connection, id: i64, app: &NewApp) -> Result<()> {
    let template_vars_json = serialize_template_vars(&app.template_vars)?;
    let rows_affected = conn
        .execute(
            "UPDATE apps SET name = ?1, working_dir = ?2, command = ?3, env_vars = ?4, auto_start = ?5, max_runtime_secs = ?6, interval_seconds = ?7, template_vars = ?8 WHERE id = ?9",
            params![
                app.name,
                app.working_dir,
                app.command,
                app.env_vars,
                app.auto_start,
                app.max_runtime_secs,
                app.interval_seconds,
                template_vars_json,
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

// ─── API Mask Operations ────────────────────────────────────────────────────

/// Insert a new API mask configuration and return its ID.
pub fn create_mask(conn: &Connection, mask: &NewMask) -> Result<i64> {
    conn.execute(
        "INSERT INTO api_masks (name, target_url, listen_port, headers, auto_start) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            mask.name,
            mask.target_url,
            mask.listen_port,
            mask.headers,
            mask.auto_start,
        ],
    )
    .context("Failed to insert mask")?;
    Ok(conn.last_insert_rowid())
}

/// Retrieve all API mask configurations.
pub fn get_all_masks(conn: &Connection) -> Result<Vec<ApiMask>> {
    let mut stmt = conn
        .prepare("SELECT id, name, target_url, listen_port, headers, auto_start, created_at FROM api_masks")
        .context("Failed to prepare get_all_masks query")?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ApiMask {
                id: row.get(0)?,
                name: row.get(1)?,
                target_url: row.get(2)?,
                listen_port: row.get::<_, i64>(3)? as u16,
                headers: row.get(4)?,
                auto_start: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .context("Failed to query masks")?;

    let mut masks = Vec::new();
    for row in rows {
        masks.push(row.context("Failed to read mask row")?);
    }
    Ok(masks)
}

/// Retrieve a single API mask configuration by its ID.
pub fn get_mask_by_id(conn: &Connection, id: i64) -> Result<Option<ApiMask>> {
    let result = conn.query_row(
        "SELECT id, name, target_url, listen_port, headers, auto_start, created_at FROM api_masks WHERE id = ?1",
        params![id],
        |row| {
            Ok(ApiMask {
                id: row.get(0)?,
                name: row.get(1)?,
                target_url: row.get(2)?,
                listen_port: row.get::<_, i64>(3)? as u16,
                headers: row.get(4)?,
                auto_start: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    );

    match result {
        Ok(mask) => Ok(Some(mask)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context(format!("Failed to get mask with id {}", id)),
    }
}

/// Retrieve a single API mask configuration by its name.
pub fn get_mask_by_name(conn: &Connection, name: &str) -> Result<Option<ApiMask>> {
    let result = conn.query_row(
        "SELECT id, name, target_url, listen_port, headers, auto_start, created_at FROM api_masks WHERE name = ?1",
        params![name],
        |row| {
            Ok(ApiMask {
                id: row.get(0)?,
                name: row.get(1)?,
                target_url: row.get(2)?,
                listen_port: row.get::<_, i64>(3)? as u16,
                headers: row.get(4)?,
                auto_start: row.get(5)?,
                created_at: row.get(6)?,
            })
        },
    );

    match result {
        Ok(mask) => Ok(Some(mask)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e).context(format!("Failed to get mask with name '{}'", name)),
    }
}

/// Update an existing API mask configuration by ID.
pub fn update_mask(conn: &Connection, id: i64, mask: &NewMask) -> Result<()> {
    let rows_affected = conn
        .execute(
            "UPDATE api_masks SET name = ?1, target_url = ?2, listen_port = ?3, headers = ?4, auto_start = ?5 WHERE id = ?6",
            params![
                mask.name,
                mask.target_url,
                mask.listen_port,
                mask.headers,
                mask.auto_start,
                id,
            ],
        )
        .context(format!("Failed to update mask with id {}", id))?;

    if rows_affected == 0 {
        anyhow::bail!("No mask found with id {}", id);
    }
    Ok(())
}

/// Delete an API mask configuration by ID.
pub fn delete_mask(conn: &Connection, id: i64) -> Result<()> {
    let rows_affected = conn
        .execute("DELETE FROM api_masks WHERE id = ?1", params![id])
        .context(format!("Failed to delete mask with id {}", id))?;

    if rows_affected == 0 {
        anyhow::bail!("No mask found with id {}", id);
    }
    Ok(())
}

/// Retrieve all API masks marked for auto-start.
pub fn get_auto_start_masks(conn: &Connection) -> Result<Vec<ApiMask>> {
    let mut stmt = conn
        .prepare("SELECT id, name, target_url, listen_port, headers, auto_start, created_at FROM api_masks WHERE auto_start = 1")
        .context("Failed to prepare get_auto_start_masks query")?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ApiMask {
                id: row.get(0)?,
                name: row.get(1)?,
                target_url: row.get(2)?,
                listen_port: row.get::<_, i64>(3)? as u16,
                headers: row.get(4)?,
                auto_start: row.get(5)?,
                created_at: row.get(6)?,
            })
        })
        .context("Failed to query auto-start masks")?;

    let mut masks = Vec::new();
    for row in rows {
        masks.push(row.context("Failed to read mask row")?);
    }
    Ok(masks)
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
            interval_seconds: None,
            template_vars: vec![],
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
            interval_seconds: None,
            template_vars: vec![],
        };
        let app2 = NewApp {
            name: "app-two".to_string(),
            working_dir: "/tmp/two".to_string(),
            command: "cmd2".to_string(),
            env_vars: "{}".to_string(),
            auto_start: true,
            max_runtime_secs: Some(60),
            interval_seconds: None,
            template_vars: vec![],
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
            interval_seconds: None,
            template_vars: vec![],
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

    // ─── Mask Tests ─────────────────────────────────────────────────────────

    fn sample_mask() -> NewMask {
        NewMask {
            name: "openai-mask".to_string(),
            target_url: "https://api.openai.com".to_string(),
            listen_port: 8080,
            headers: r#"{"Authorization":"Bearer sk-test"}"#.to_string(),
            auto_start: false,
        }
    }

    #[test]
    fn test_create_mask_and_retrieve() {
        let conn = init_memory().unwrap();
        let mask = sample_mask();
        let id = create_mask(&conn, &mask).unwrap();
        assert!(id > 0);

        let retrieved = get_mask_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(retrieved.name, "openai-mask");
        assert_eq!(retrieved.target_url, "https://api.openai.com");
        assert_eq!(retrieved.listen_port, 8080);
        assert_eq!(retrieved.headers, r#"{"Authorization":"Bearer sk-test"}"#);
        assert!(!retrieved.auto_start);
    }

    #[test]
    fn test_get_mask_by_name() {
        let conn = init_memory().unwrap();
        let mask = sample_mask();
        create_mask(&conn, &mask).unwrap();

        let retrieved = get_mask_by_name(&conn, "openai-mask").unwrap().unwrap();
        assert_eq!(retrieved.target_url, "https://api.openai.com");

        let not_found = get_mask_by_name(&conn, "nonexistent").unwrap();
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_all_masks() {
        let conn = init_memory().unwrap();

        let mask1 = NewMask {
            name: "mask-one".to_string(),
            target_url: "https://api.one.com".to_string(),
            listen_port: 8080,
            headers: "{}".to_string(),
            auto_start: false,
        };
        let mask2 = NewMask {
            name: "mask-two".to_string(),
            target_url: "https://api.two.com".to_string(),
            listen_port: 8081,
            headers: "{}".to_string(),
            auto_start: true,
        };

        create_mask(&conn, &mask1).unwrap();
        create_mask(&conn, &mask2).unwrap();

        let all = get_all_masks(&conn).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].name, "mask-one");
        assert_eq!(all[1].name, "mask-two");
    }

    #[test]
    fn test_update_mask() {
        let conn = init_memory().unwrap();
        let mask = sample_mask();
        let id = create_mask(&conn, &mask).unwrap();

        let updated = NewMask {
            name: "updated-mask".to_string(),
            target_url: "https://api.anthropic.com".to_string(),
            listen_port: 9090,
            headers: r#"{"x-api-key":"sk-new"}"#.to_string(),
            auto_start: true,
        };
        update_mask(&conn, id, &updated).unwrap();

        let retrieved = get_mask_by_id(&conn, id).unwrap().unwrap();
        assert_eq!(retrieved.name, "updated-mask");
        assert_eq!(retrieved.target_url, "https://api.anthropic.com");
        assert_eq!(retrieved.listen_port, 9090);
        assert!(retrieved.auto_start);
    }

    #[test]
    fn test_delete_mask() {
        let conn = init_memory().unwrap();
        let mask = sample_mask();
        let id = create_mask(&conn, &mask).unwrap();

        delete_mask(&conn, id).unwrap();

        let result = get_mask_by_id(&conn, id).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_mask_unique_constraint_on_name() {
        let conn = init_memory().unwrap();
        let mask = sample_mask();
        create_mask(&conn, &mask).unwrap();

        let duplicate = sample_mask();
        let result = create_mask(&conn, &duplicate);
        assert!(result.is_err());
    }

    #[test]
    fn test_get_auto_start_masks() {
        let conn = init_memory().unwrap();

        let mask1 = NewMask {
            name: "no-auto".to_string(),
            target_url: "https://api.one.com".to_string(),
            listen_port: 8080,
            headers: "{}".to_string(),
            auto_start: false,
        };
        let mask2 = NewMask {
            name: "yes-auto".to_string(),
            target_url: "https://api.two.com".to_string(),
            listen_port: 8081,
            headers: "{}".to_string(),
            auto_start: true,
        };

        create_mask(&conn, &mask1).unwrap();
        create_mask(&conn, &mask2).unwrap();

        let auto_masks = get_auto_start_masks(&conn).unwrap();
        assert_eq!(auto_masks.len(), 1);
        assert_eq!(auto_masks[0].name, "yes-auto");
    }
}
