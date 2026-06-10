# TICKET-16: API Mask Database Model

## Priority: High
## Dependencies: TICKET-15
## Blocks: TICKET-17, TICKET-18, TICKET-19

## Description
Add the `api_masks` table to the SQLite schema and implement CRUD operations. The table stores mask configuration: name, target URL, listen port, headers (JSON), and auto_start flag.

Migration is automatic — `CREATE TABLE IF NOT EXISTS` runs on every startup, so existing installs get the new table on next launch.

## Acceptance Criteria
- [ ] `api_masks` table added to SCHEMA constant in `src/db/mod.rs`
- [ ] `ApiMask` struct in `src/db/models.rs` with fields: id, name, target_url, listen_port, headers (String/JSON), auto_start, created_at
- [ ] `NewMask` struct for inserts (no id, no created_at)
- [ ] CRUD operations in `src/db/operations.rs`:
  - `insert_mask(conn, new_mask) -> Result<i64>`
  - `get_all_masks(conn) -> Result<Vec<ApiMask>>`
  - `get_mask_by_id(conn, id) -> Result<Option<ApiMask>>`
  - `get_mask_by_name(conn, name) -> Result<Option<ApiMask>>`
  - `update_mask(conn, id, new_mask) -> Result<()>`
  - `delete_mask(conn, id) -> Result<()>`
  - `get_auto_start_masks(conn) -> Result<Vec<ApiMask>>`
- [ ] Unit tests for all CRUD operations

## Files
- `src/db/mod.rs` (schema update)
- `src/db/models.rs` (ApiMask, NewMask structs)
- `src/db/operations.rs` (CRUD functions)

## Tests
- `cargo test` — all existing tests still pass
- New tests verify insert, read, update, delete, and auto_start filtering
- Verify idempotent schema creation (existing DBs get new table without data loss)
