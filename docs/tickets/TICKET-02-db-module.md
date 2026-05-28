# TICKET-02: Database Module

## Priority: High
## Dependencies: TICKET-01
## Blocks: TICKET-04, TICKET-07, TICKET-08, TICKET-12

## Description
Implement SQLite database layer using `rusqlite`. Handles schema creation, migrations, and CRUD operations for the `apps` and `config` tables.

## Acceptance Criteria
- [ ] Schema creation on first run (apps table + config table)
- [ ] DB file location: `~/.local/share/apprunner/apprunner.db`
- [ ] CRUD for apps: create, read_all, read_by_id, update, delete
- [ ] Config table with global defaults (global_max_runtime_secs = 18000)
- [ ] Get/set config values
- [ ] Proper error handling with `anyhow`

## Data Model

```sql
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
```

## Files
- `src/db/mod.rs`

## Tests
- Unit tests for all CRUD operations (using in-memory SQLite)
- Test schema creation
- Test default config insertion
- Test unique constraint on app name
- Test update and delete operations
